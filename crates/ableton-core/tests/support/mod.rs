//! A fake AbletonJS Remote Script speaking the same protocol as `Socket.py`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use ableton_core::AbletonOptions;

/// Answers one command with its `data`, or `Err(message)` for a failed slot.
pub type Handler = dyn Fn(&Value) -> Result<Value, String> + Send + Sync;

pub struct FakeLive {
    pub port: u16,
    envelopes: tokio::sync::Mutex<mpsc::UnboundedReceiver<Value>>,
    connection: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    /// Envelope replies are held back while this is set.
    pub mute: Arc<Mutex<bool>>,
}

impl FakeLive {
    pub async fn start(
        handler: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static,
    ) -> Self {
        Self::start_with_password(None, handler).await
    }

    pub async fn start_with_password(
        password: Option<&str>,
        handler: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (envelopes_tx, envelopes) = mpsc::unbounded_channel();
        let connection = Arc::new(Mutex::new(None));
        let mute = Arc::new(Mutex::new(false));
        let handler: Arc<Handler> = Arc::new(handler);
        let password = password.map(str::to_owned);

        let server_connection = Arc::clone(&connection);
        let server_mute = Arc::clone(&mute);
        tokio::spawn(async move {
            while let Ok((tcp, _)) = listener.accept().await {
                let Ok(socket) = accept_async(tcp).await else {
                    continue;
                };
                tokio::spawn(serve(
                    socket,
                    port,
                    password.clone(),
                    Arc::clone(&handler),
                    envelopes_tx.clone(),
                    Arc::clone(&server_connection),
                    Arc::clone(&server_mute),
                ));
            }
        });

        Self {
            port,
            envelopes: tokio::sync::Mutex::new(envelopes),
            connection,
            mute,
        }
    }

    pub fn options(&self) -> AbletonOptions {
        AbletonOptions {
            port: self.port,
            ..Default::default()
        }
    }

    /// The next envelope the client sent.
    pub async fn next_envelope(&self) -> Value {
        tokio::time::timeout(Duration::from_secs(5), self.envelopes.lock().await.recv())
            .await
            .expect("no envelope within 5s")
            .expect("server stopped")
    }

    /// Pushes a listener event to the connected client.
    pub fn push(&self, event: &str, data: Value) {
        send(&self.connection, message(event, data, Value::Null));
    }

    /// Drops the current connection from the server side.
    pub fn disconnect(&self) {
        if let Some(tx) = self.connection.lock().unwrap().take() {
            let _ = tx.send(Message::Close(None));
        }
    }
}

async fn serve(
    socket: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    port: u16,
    password: Option<String>,
    handler: Arc<Handler>,
    envelopes: mpsc::UnboundedSender<Value>,
    connection: Arc<Mutex<Option<mpsc::UnboundedSender<Message>>>>,
    mute: Arc<Mutex<bool>>,
) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    *connection.lock().unwrap() = Some(tx.clone());

    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let closing = matches!(message, Message::Close(_));
            if sink.send(message).await.is_err() || closing {
                break;
            }
        }
        let _ = sink.close().await;
    });

    let salt = "fake-salt";
    let mut connect = json!({ "port": port });
    if password.is_some() {
        connect["requiresAuth"] = json!(true);
        connect["salt"] = json!(salt);
    }
    let _ = tx.send(message("connect", connect, Value::Null));

    let mut authenticated = password.is_none();
    while let Some(Ok(frame)) = stream.next().await {
        let Message::Text(text) = frame else {
            if matches!(frame, Message::Close(_)) {
                break;
            }
            continue;
        };
        let envelope: Value = serde_json::from_str(text.as_str()).unwrap();
        let uuid = envelope["uuid"].clone();
        let _ = envelopes.send(envelope.clone());

        let commands = envelope["commands"].as_array().cloned().unwrap_or_default();
        if !authenticated {
            let first = &commands[0];
            let expected = hmac_hex(password.as_deref().unwrap(), salt);
            if first["ns"] != "internal" || first["name"] != "authenticate" {
                let _ = tx.send(message("error", json!("Unauthorized"), uuid));
                continue;
            }
            if first["args"]["hash"] != expected {
                let _ = tx.send(message("error", json!("Invalid password"), uuid));
                let _ = tx.send(Message::Close(None));
                break;
            }
            authenticated = true;
        }

        if *mute.lock().unwrap() {
            continue;
        }

        let results: Vec<Value> = commands
            .iter()
            .map(|command| {
                if command["name"] == "authenticate" {
                    return json!({ "ok": true, "data": null });
                }
                match handler(command) {
                    Ok(data) => json!({ "ok": true, "data": data }),
                    Err(error) => {
                        json!({ "ok": false, "error": error, "errorType": "RuntimeError" })
                    }
                }
            })
            .collect();
        let _ = tx.send(message("result", json!(results), uuid));
    }
}

fn message(event: &str, data: Value, uuid: Value) -> Message {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    Message::Text(
        json!({ "event": event, "data": data, "uuid": uuid, "ts": ts })
            .to_string()
            .into(),
    )
}

fn send(connection: &Mutex<Option<mpsc::UnboundedSender<Message>>>, message: Message) {
    if let Some(tx) = connection.lock().unwrap().as_ref() {
        let _ = tx.send(message);
    }
}

fn hmac_hex(password: &str, salt: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(password.as_bytes()).unwrap();
    mac.update(salt.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Answers `get_prop` for a few song properties and pings.
pub fn default_handler(command: &Value) -> Result<Value, String> {
    let prop = command["args"]["prop"].as_str().unwrap_or_default();
    match (
        command["ns"].as_str().unwrap(),
        command["name"].as_str().unwrap(),
        prop,
    ) {
        ("internal", "get_prop", "ping") => Ok(json!(true)),
        ("internal", "get_prop", "version") => Ok(json!(ableton_core::PLUGIN_VERSION)),
        ("song", "get_prop", "tempo") => Ok(json!(120.0)),
        ("song", "get_prop", "is_playing") => Ok(json!(false)),
        ("song", "set_prop", _) => Ok(Value::Null),
        _ => Err(format!("unhandled command {command}")),
    }
}
