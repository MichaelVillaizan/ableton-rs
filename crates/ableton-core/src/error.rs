//! Errors returned by the client.

use std::sync::Arc;

use crate::protocol::CommandEnvelope;

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Everything that can go wrong while talking to Live.
///
/// The type is cheap to clone so a single failure can be handed to every
/// caller waiting on the same connection or command batch.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A command was sent before [`crate::Ableton::start`] was called.
    #[error("The client hasn't been started yet. Please call start() first.")]
    NotStarted,

    /// A command was sent while there was no open socket to Live.
    #[error("The client is disconnected.")]
    NotConnected,

    /// [`crate::Ableton::start`] didn't manage to connect within its timeout.
    #[error("Connection timed out.")]
    ConnectionTimeout,

    /// The client was closed while something was still waiting on it.
    #[error("The client was closed before the operation could complete.")]
    Closed,

    /// A command envelope didn't get a reply within the command timeout.
    #[error("{message}")]
    Timeout {
        message: String,
        payload: Arc<CommandEnvelope>,
    },

    /// Live disconnected while a command envelope was waiting for its reply.
    #[error("{message}")]
    Disconnected {
        message: String,
        payload: Arc<CommandEnvelope>,
    },

    /// A command failed inside the Remote Script. `error_type` is the name of
    /// the Python exception class (e.g. `RuntimeError`) when the plugin sent it.
    #[error("{message}")]
    Command {
        message: String,
        error_type: Option<String>,
    },

    /// The Remote Script rejected a whole envelope (e.g. `Unauthorized`).
    #[error("{0}")]
    Remote(String),

    /// Authenticating with a password-protected Remote Script failed.
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Live sent something that doesn't follow the protocol.
    #[error("Unexpected response from Ableton: {0}")]
    Protocol(String),

    /// A value from Live couldn't be converted into the requested Rust type.
    #[error("Failed to decode value: {0}")]
    Decode(String),

    /// A value couldn't be encoded as JSON.
    #[error("Failed to encode value: {0}")]
    Encode(String),

    /// A value passed to a helper was invalid (e.g. a malformed color).
    #[error("{0}")]
    InvalidArgument(String),
}

impl Error {
    pub(crate) fn decode(expected: &str, got: &serde_json::Value) -> Self {
        let mut got = got.to_string();
        if got.len() > 100 {
            got.truncate(100);
            got.push('…');
        }
        Error::Decode(format!("expected {expected}, got {got}"))
    }
}
