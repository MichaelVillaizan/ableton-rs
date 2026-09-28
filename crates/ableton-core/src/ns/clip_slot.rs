use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Clip, IntoLiveId};
use crate::error::Result;
use crate::types::Color;

string_enum! {
    /// Playback state of a clip slot.
    pub enum PlayingStatus {
        Stopped = "stopped",
        Started = "started",
        Recording = "recording",
    }
}

/// A clip slot as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawClipSlot {
    pub id: String,
    pub color: Option<Color>,
    pub has_clip: bool,
    pub is_playing: bool,
    pub is_recording: bool,
    pub is_triggered: bool,
}

live_object! {
    /// An entry of Live's Session View matrix.
    object ClipSlot { ns: "clip_slot", raw: RawClipSlot }
}

props! { ClipSlot {
    clip: Option<Clip> [cached];
    color: Option<Color> [get set(Color) observe];
    color_index: i32 [get observe];
    controls_other_clips: bool [get observe];
    has_clip: bool [get observe];
    has_stop_button: bool [get observe];
    is_group_slot: bool [get];
    is_playing: bool [get];
    is_recording: bool [get];
    is_triggered: bool [get observe];
    playing_status: PlayingStatus [get observe];
    will_record_on_start: bool [get];
}}

impl ClipSlot {
    /// Creates an empty clip of the given length. Fails on slots that
    /// already have a clip or that aren't on a MIDI track.
    pub async fn create_clip(&self, length: f64) -> Result<()> {
        self.handle.call("create_clip", json!([length])).await
    }

    /// Removes the clip. Fails when the slot is empty.
    pub async fn delete_clip(&self) -> Result<()> {
        self.handle.call("delete_clip", None).await
    }

    /// Duplicates the clip into `target`, replacing whatever is there. Fails
    /// when the slot is empty, the track types differ or either slot is a
    /// group slot.
    pub async fn duplicate_clip_to(&self, target: impl IntoLiveId) -> Result<()> {
        self.handle
            .call(
                "duplicate_clip_to",
                json!({ "slot_id": target.into_live_id() }),
            )
            .await
    }

    /// Fires the clip, or the stop button when the slot is empty.
    pub async fn fire(&self) -> Result<()> {
        self.handle.call("fire", None).await
    }

    /// Sets the state of the slot's launch button directly. Supports all
    /// launch modes.
    pub async fn set_fire_button_state(&self, state: bool) -> Result<()> {
        self.handle
            .call("set_fire_button_state", json!([state]))
            .await
    }

    /// Stops the clip if it's playing.
    pub async fn stop(&self) -> Result<()> {
        self.handle.call("stop", None).await
    }
}
