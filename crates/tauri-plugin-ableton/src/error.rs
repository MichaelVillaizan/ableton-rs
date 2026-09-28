use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned to the webview.
///
/// Serialized as `{ kind, message, errorType }`, where `kind` names the
/// variant of [`ableton_core::Error`] (e.g. `"timeout"`, `"command"`).
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ableton(#[from] ableton_core::Error),

    #[error("Unknown listener {0}")]
    UnknownListener(u64),
}

impl Error {
    fn kind(&self) -> &'static str {
        use ableton_core::Error as E;
        match self {
            Error::UnknownListener(_) => "unknownListener",
            Error::Ableton(error) => match error {
                E::NotStarted => "notStarted",
                E::NotConnected => "notConnected",
                E::ConnectionTimeout => "connectionTimeout",
                E::Closed => "closed",
                E::Timeout { .. } => "timeout",
                E::Disconnected { .. } => "disconnected",
                E::Command { .. } => "command",
                E::Remote(_) => "remote",
                E::Auth(_) => "auth",
                E::Protocol(_) => "protocol",
                E::Decode(_) => "decode",
                E::Encode(_) => "encode",
                E::InvalidArgument(_) => "invalidArgument",
                _ => "other",
            },
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let error_type = match self {
            Error::Ableton(ableton_core::Error::Command { error_type, .. }) => {
                error_type.as_deref()
            }
            _ => None,
        };

        let mut state = serializer.serialize_struct("Error", 3)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("errorType", &error_type)?;
        state.end()
    }
}
