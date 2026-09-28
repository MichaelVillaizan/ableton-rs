use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A cue point as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawCuePoint {
    pub id: String,
    pub name: String,
    pub time: f64,
}

live_object! {
    /// A cue point (locator) in the arrangement.
    object CuePoint { ns: "cue-point", raw: RawCuePoint }
}

props! { CuePoint {
    name: String [get set observe];
    time: f64 [get observe];
}}

impl CuePoint {
    /// Jumps to this cue point (quantized) while playing, or moves the start
    /// position here while stopped.
    pub async fn jump(&self) -> Result<()> {
        self.handle.call("jump", None).await
    }
}
