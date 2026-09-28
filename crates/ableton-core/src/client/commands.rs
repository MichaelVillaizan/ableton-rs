//! Command batching, reply matching and incoming message routing.

use std::sync::{Arc, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::sync::{Semaphore, mpsc, oneshot};
use tracing::{debug, warn};

use super::Inner;
use crate::error::{Error, Result};
use crate::events::{ClientEvent, DisconnectKind};
use crate::protocol::{Command, CommandEnvelope, CommandSlotResult};

/// Maximum number of envelopes waiting for a reply at the same time.
const MAX_ENVELOPES_IN_FLIGHT: usize = 200;

/// A command waiting to be put into an envelope.
pub(crate) struct QueuedCommand {
    pub command: Command,
    pub reply: oneshot::Sender<Result<Value>>,
}

/// An envelope waiting for its reply.
pub(crate) struct PendingReply {
    tx: oneshot::Sender<Result<Value>>,
    payload: Arc<CommandEnvelope>,
}

impl PendingReply {
    /// Fails the envelope because Live went away.
    pub fn disconnect(self) {
        let message = format!(
            "Live disconnected before being able to respond to {}",
            self.payload.summary()
        );
        let _ = self.tx.send(Err(Error::Disconnected {
            message,
            payload: self.payload,
        }));
    }
}

/// Coalesces commands issued at the same time into one envelope.
///
/// After the first command arrives the batcher yields once, so commands
/// issued concurrently (e.g. through `join!`) can join the same envelope,
/// then sends everything that is queued.
pub(crate) async fn run_batcher(
    inner: Weak<Inner>,
    mut queue: mpsc::UnboundedReceiver<QueuedCommand>,
) {
    let in_flight = Arc::new(Semaphore::new(MAX_ENVELOPES_IN_FLIGHT));

    while let Some(first) = queue.recv().await {
        tokio::task::yield_now().await;

        let mut batch = vec![first];
        while let Ok(next) = queue.try_recv() {
            batch.push(next);
        }

        let Some(inner) = inner.upgrade() else {
            for entry in batch {
                let _ = entry.reply.send(Err(Error::Closed));
            }
            return;
        };

        let permit = Arc::clone(&in_flight)
            .acquire_owned()
            .await
            .expect("the semaphore is never closed");
        tokio::spawn(async move {
            inner.flush_batch(batch).await;
            drop(permit);
        });
    }
}

/// Splits the reply of an envelope into one result per command.
pub(crate) fn split_results(value: Value, expected: usize) -> Result<Vec<Result<Value>>> {
    let unexpected = || Error::Protocol("Unexpected commands response from Ableton.".into());

    let Value::Array(slots) = value else {
        return Err(unexpected());
    };
    if slots.len() != expected {
        return Err(unexpected());
    }

    slots
        .into_iter()
        .map(|slot| {
            let slot: CommandSlotResult = serde_json::from_value(slot).map_err(|_| unexpected())?;
            Ok(if slot.ok {
                Ok(slot.data)
            } else {
                Err(Error::Command {
                    message: slot
                        .error
                        .filter(|message| !message.is_empty())
                        .unwrap_or_else(|| "Command failed".into()),
                    error_type: slot.error_type,
                })
            })
        })
        .collect()
}

impl Inner {
    async fn flush_batch(&self, batch: Vec<QueuedCommand>) {
        if batch.len() > 1 {
            debug!(length = batch.len(), "Flushing command queue");
        }

        let (commands, replies): (Vec<_>, Vec<_>) = batch
            .into_iter()
            .map(|entry| (entry.command, entry.reply))
            .unzip();
        let expected = replies.len();

        match self
            .send_envelope(commands)
            .await
            .and_then(|value| split_results(value, expected))
        {
            Ok(results) => {
                for (reply, result) in replies.into_iter().zip(results) {
                    let _ = reply.send(result);
                }
            }
            Err(error) => {
                for reply in replies {
                    let _ = reply.send(Err(error.clone()));
                }
            }
        }
    }

    /// Sends one envelope and waits for its reply.
    pub(super) async fn send_envelope(&self, commands: Vec<Command>) -> Result<Value> {
        // The longest timeout requested by any of the commands wins.
        let timeout = commands
            .iter()
            .filter_map(|command| command.timeout)
            .map(Duration::from_millis)
            .fold(self.options.command_timeout, Duration::max);

        let envelope = Arc::new(CommandEnvelope {
            uuid: self.next_id(),
            commands,
        });
        let message =
            serde_json::to_string(&*envelope).map_err(|error| Error::Encode(error.to_string()))?;

        let (tx, rx) = oneshot::channel();
        self.pending().insert(
            envelope.uuid.clone(),
            PendingReply {
                tx,
                payload: Arc::clone(&envelope),
            },
        );

        let sent_at = Instant::now();
        if let Err(error) = self.send_raw(message) {
            self.pending().remove(&envelope.uuid);
            return Err(error);
        }

        let result = match tokio::time::timeout(timeout, rx).await {
            Ok(reply) => reply.unwrap_or(Err(Error::Closed)),
            Err(_) => {
                self.pending().remove(&envelope.uuid);
                Err(Error::Timeout {
                    message: format!(
                        "The command {} timed out after {} ms.",
                        envelope.summary(),
                        timeout.as_millis()
                    ),
                    payload: Arc::clone(&envelope),
                })
            }
        };

        let duration = sent_at.elapsed();
        if result.is_ok() && duration > self.options.command_warn {
            warn!(
                commands = %envelope.summary(),
                duration_ms = duration.as_millis() as u64,
                "Commands took longer than expected"
            );
        }
        result
    }

    /// Routes a text frame received from Live.
    pub(super) fn handle_incoming(self: &Arc<Self>, text: &str) {
        self.emit(ClientEvent::RawMessage(Arc::from(text)));

        let mut message: Value = match serde_json::from_str(text) {
            Ok(message) => message,
            Err(error) => {
                self.emit(ClientEvent::Error(Error::Protocol(format!(
                    "invalid JSON message: {error}"
                ))));
                return;
            }
        };
        self.touch();

        let event = message
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let uuid = message
            .get("uuid")
            .and_then(Value::as_str)
            .map(str::to_owned);

        if let Some(sent_at) = message.get("ts").and_then(Value::as_f64) {
            let latency = unix_millis() - sent_at;
            if latency > self.options.message_latency_warn.as_millis() as f64 {
                warn!(
                    event,
                    latency_ms = latency,
                    "Message arrived with high latency"
                );
            }
        }

        if self.events.receiver_count() > 0 {
            self.emit(ClientEvent::Message(Arc::new(message.clone())));
        }

        let data = message
            .get_mut("data")
            .map(Value::take)
            .unwrap_or(Value::Null);

        match event.as_str() {
            "result" | "error" => {
                let pending = uuid.as_deref().and_then(|uuid| self.pending().remove(uuid));
                if let Some(pending) = pending {
                    let result = if event == "result" {
                        Ok(data)
                    } else {
                        Err(Error::Remote(value_to_message(&data)))
                    };
                    let _ = pending.tx.send(result);
                }
            }
            "disconnect" => self.request_socket_drop(DisconnectKind::Realtime),
            "connect" => {
                tokio::spawn(Arc::clone(self).handle_server_connect(data));
            }
            _ => {
                if !self.listeners.dispatch(&event, &data) && uuid.is_some() {
                    warn!(
                        message = text,
                        "Message could not be assigned to any request"
                    );
                }
            }
        }
    }
}

fn value_to_message(value: &Value) -> String {
    match value {
        Value::String(message) => message.clone(),
        other => other.to_string(),
    }
}

fn unix_millis() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |duration| duration.as_secs_f64() * 1000.0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn splits_slot_results() {
        let results = split_results(
            json!([
                { "ok": true, "data": 120.0 },
                { "ok": false, "error": "boom", "errorType": "RuntimeError" },
                { "ok": false }
            ]),
            3,
        )
        .unwrap();

        assert_eq!(results[0].as_ref().unwrap(), &json!(120.0));
        match &results[1] {
            Err(Error::Command {
                message,
                error_type,
            }) => {
                assert_eq!(message, "boom");
                assert_eq!(error_type.as_deref(), Some("RuntimeError"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(
            matches!(&results[2], Err(Error::Command { message, .. }) if message == "Command failed")
        );
    }

    #[test]
    fn rejects_mismatched_slot_counts() {
        assert!(split_results(json!([]), 1).is_err());
        assert!(split_results(json!({}), 0).is_err());
    }
}
