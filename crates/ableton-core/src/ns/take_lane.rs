use serde::{Deserialize, Serialize};
use serde_json::json;

use super::Clip;
use crate::error::Result;

/// A take lane as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawTakeLane {
    pub id: String,
    pub name: String,
}

live_object! {
    /// A take lane of a track (Arrangement View comping).
    object TakeLane { ns: "take-lane", raw: RawTakeLane }
}

props! { TakeLane {
    arrangement_clips: Vec<Clip> [cached observe];
    name: String [get set observe];
}}

impl TakeLane {
    /// Creates an audio clip referencing `file_path` at `start_time`.
    pub async fn create_audio_clip(&self, file_path: &str, start_time: f64) -> Result<Clip> {
        self.handle
            .call(
                "create_audio_clip",
                json!({ "file_path": file_path, "start_time": start_time }),
            )
            .await
    }

    /// Creates an empty MIDI clip at `start_time`.
    pub async fn create_midi_clip(&self, start_time: f64, length: f64) -> Result<Clip> {
        self.handle
            .call(
                "create_midi_clip",
                json!({ "start_time": start_time, "length": length }),
            )
            .await
    }
}
