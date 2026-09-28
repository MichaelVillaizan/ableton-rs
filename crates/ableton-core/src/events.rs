//! Client lifecycle events, delivered through [`crate::Ableton::events`].

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::Error;

/// Why the connection to Live was considered lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DisconnectKind {
    /// The socket closed or Live announced that it's going away.
    Realtime,
    /// Live stopped answering heartbeat pings.
    Heartbeat,
}

/// How the connection to Live was (re-)established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectKind {
    /// The Remote Script greeted a new socket.
    Realtime,
    /// A heartbeat got an answer again after Live was unresponsive.
    Heartbeat,
    /// The initial connection made by [`crate::Ableton::start`].
    Start,
}

/// An event emitted by the client.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ClientEvent {
    /// Live is reachable.
    Connect(ConnectKind),
    /// Live is no longer reachable. All property listeners were dropped;
    /// register them again after the next [`ClientEvent::Connect`].
    Disconnect(DisconnectKind),
    /// A text frame exactly as it was received.
    RawMessage(Arc<str>),
    /// A parsed message (`{ event, data, uuid, ts }`).
    Message(Arc<Value>),
    /// Something went wrong outside of a specific command.
    Error(Error),
    /// Round-trip time of the last heartbeat.
    Ping(Duration),
}
