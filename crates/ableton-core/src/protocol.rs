//! Wire format shared with the AbletonJS Remote Script.
//!
//! The client sends a [`CommandEnvelope`] per text frame. The script answers
//! with a message `{ event, data, uuid, ts }` where `event` is `"result"`
//! (`data` holds one `{ ok, data | error, errorType }` slot per command), `"error"` (the whole
//! envelope failed), `"connect"`/`"disconnect"`, or the event id of a
//! property listener.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single command addressed to a Live object.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Command {
    /// Namespace handling the command (e.g. `"song"`, `"track"`).
    pub ns: String,
    /// Id of the Live object, when addressing a specific one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsid: Option<String>,
    /// Function to run (e.g. `"get_prop"`, `"fire"`).
    pub name: String,
    /// Etag of a previously cached response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Ask the script to wrap the result as `{ data, etag }`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,
    /// Timeout for the envelope carrying this command, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Arguments: an object for keyword arguments, an array for positional ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
}

impl Command {
    /// Creates a command without arguments.
    pub fn new(ns: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            ns: ns.into(),
            nsid: None,
            name: name.into(),
            etag: None,
            cache: None,
            timeout: None,
            args: None,
        }
    }

    /// Addresses a specific Live object.
    pub fn nsid(mut self, nsid: Option<impl Into<String>>) -> Self {
        self.nsid = nsid.map(Into::into);
        self
    }

    /// Sets the arguments.
    pub fn args(mut self, args: Value) -> Self {
        self.args = Some(args);
        self
    }

    /// Overrides the reply timeout of the envelope carrying this command.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout.as_millis().try_into().unwrap_or(u64::MAX));
        self
    }

    fn is_authenticate(&self) -> bool {
        self.ns == "internal" && self.name == "authenticate"
    }
}

impl fmt::Display for Command {
    /// `ns.name(args)`, with long arguments truncated and secrets hidden.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_authenticate() {
            return write!(f, "{}.{}({{ hash: *** }})", self.ns, self.name);
        }

        let mut args = self
            .args
            .as_ref()
            .map_or_else(|| "undefined".to_owned(), Value::to_string);
        if args.chars().count() > 100 {
            args = args.chars().take(97).collect::<String>() + "...";
        }
        write!(f, "{}.{}({})", self.ns, self.name, args)
    }
}

/// The payload of one outgoing text frame.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandEnvelope {
    /// Id the script echoes back on the reply.
    pub uuid: String,
    /// Commands executed in order by the script.
    pub commands: Vec<Command>,
}

impl CommandEnvelope {
    /// Short human-readable description used in logs and errors.
    pub fn summary(&self) -> String {
        match self.commands.as_slice() {
            [] => "commands[0]".to_owned(),
            [only] => only.to_string(),
            [first, ..] => format!("commands[{}] starting with {first}", self.commands.len()),
        }
    }
}

/// Outcome of a single command inside a `"result"` reply.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CommandSlotResult {
    pub ok: bool,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default, rename = "errorType")]
    pub error_type: Option<String>,
}

/// Payload of a `"connect"` message.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ConnectData {
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default, rename = "requiresAuth")]
    pub requires_auth: bool,
    #[serde(default)]
    pub salt: Option<String>,
}
