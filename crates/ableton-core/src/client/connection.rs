//! The connection task: owns the WebSocket and reconnects with backoff.

use std::sync::Weak;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, info, warn};

use super::Inner;
use crate::events::DisconnectKind;

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(250);
const MAX_RECONNECT_DELAY: Duration = Duration::from_millis(2000);
/// How long a graceful close waits for queued frames and the close handshake.
const CLOSE_GRACE_PERIOD: Duration = Duration::from_secs(2);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Instructions for the connection task.
#[derive(Debug)]
pub(crate) enum Control {
    /// Drop the current socket (if any) and reconnect.
    Drop(DisconnectKind),
    /// Close the socket gracefully and stop.
    Shutdown,
}

/// What ended a socket's session.
enum Exit {
    Reconnect,
    Shutdown,
}

pub(crate) async fn run(inner: Weak<Inner>, mut control: mpsc::UnboundedReceiver<Control>) {
    let mut delay = INITIAL_RECONNECT_DELAY;

    loop {
        let (url, connect_timeout) = match inner.upgrade() {
            Some(inner) if inner.should_reconnect() => {
                (inner.options.url(), inner.options.connect_timeout)
            }
            _ => return,
        };

        let attempt = tokio::select! {
            attempt = tokio::time::timeout(connect_timeout, connect_async(url.as_str())) => attempt,
            instruction = control.recv() => match instruction {
                // There is no socket to drop yet, just try again.
                Some(Control::Drop(_)) => continue,
                Some(Control::Shutdown) | None => return,
            },
        };

        match attempt {
            Ok(Ok((socket, _))) => {
                delay = INITIAL_RECONNECT_DELAY;
                if let Exit::Shutdown = run_socket(&inner, socket, &mut control).await {
                    return;
                }
            }
            Ok(Err(error)) => {
                debug!(%url, %error, "WebSocket connection failed");
                if let Some(inner) = inner.upgrade() {
                    inner.handle_disconnect(DisconnectKind::Realtime);
                }
            }
            Err(_) => {
                warn!(%url, timeout_ms = connect_timeout.as_millis() as u64, "WebSocket connection timed out");
                if let Some(inner) = inner.upgrade() {
                    inner.handle_disconnect(DisconnectKind::Realtime);
                }
            }
        }

        match inner.upgrade() {
            Some(inner) if inner.should_reconnect() => {}
            _ => return,
        }

        info!(delay_ms = delay.as_millis() as u64, %url, "Reconnecting to Live");
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            instruction = control.recv() => match instruction {
                Some(Control::Drop(_)) => {}
                Some(Control::Shutdown) | None => return,
            },
        }
        delay = (delay * 2).min(MAX_RECONNECT_DELAY);
    }
}

/// Pumps one open socket until it closes or we're told to drop it.
async fn run_socket(
    inner: &Weak<Inner>,
    socket: Socket,
    control: &mut mpsc::UnboundedReceiver<Control>,
) -> Exit {
    let (mut sink, mut stream) = socket.split();

    // Outgoing frames go through a queue so that sending never blocks the
    // caller. The writer flushes the whole queue before a close frame goes
    // out, so nothing that was already sent gets lost when closing.
    let (outgoing, mut queue) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(message) = queue.recv().await {
            let closing = matches!(message, Message::Close(_));
            if let Err(error) = sink.send(message).await {
                debug!(%error, "Failed to write to the WebSocket");
                return;
            }
            if closing {
                break;
            }
        }
        let _ = sink.close().await;
    });

    match inner.upgrade() {
        Some(inner) => inner.attach_socket(outgoing.clone()),
        None => return Exit::Shutdown,
    }

    let (exit, kind) = loop {
        tokio::select! {
            frame = stream.next() => match frame {
                Some(Ok(Message::Text(text))) => match inner.upgrade() {
                    Some(inner) => inner.handle_incoming(text.as_str()),
                    None => break (Exit::Shutdown, DisconnectKind::Realtime),
                },
                Some(Ok(Message::Close(_))) | None => break (Exit::Reconnect, DisconnectKind::Realtime),
                Some(Ok(_)) => {}
                Some(Err(error)) => {
                    debug!(%error, "WebSocket read failed");
                    break (Exit::Reconnect, DisconnectKind::Realtime);
                }
            },
            instruction = control.recv() => match instruction {
                Some(Control::Drop(kind)) => break (Exit::Reconnect, kind),
                Some(Control::Shutdown) | None => break (Exit::Shutdown, DisconnectKind::Realtime),
            },
        }
    };

    // New sends fail fast from here on; frames already queued still go out
    // ahead of the close frame.
    if let Some(inner) = inner.upgrade() {
        inner.detach_socket();
    }
    let _ = outgoing.send(Message::Close(None));
    drop(outgoing);

    if let Exit::Shutdown = exit {
        let _ = tokio::time::timeout(CLOSE_GRACE_PERIOD, writer).await;
        // Wait for the other side to acknowledge the close.
        let _ = tokio::time::timeout(CLOSE_GRACE_PERIOD, async {
            while let Some(Ok(frame)) = stream.next().await {
                if matches!(frame, Message::Close(_)) {
                    break;
                }
            }
        })
        .await;
    }

    if let Some(inner) = inner.upgrade() {
        inner.handle_disconnect(kind);
    }
    exit
}
