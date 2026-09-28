use serde_json::json;

use super::device::device_members;
use super::{Chain, DeviceView, IntoLiveId, RawDevice};
use crate::error::Result;

/// Live's `class_name` of the Looper audio effect.
pub const LOOPER_CLASS_NAME: &str = "Looper";

live_object! {
    /// Live's Looper audio effect.
    object LooperDevice { ns: "looper-device", raw: RawDevice }
}

device_members!(LooperDevice);

props! { LooperDevice {
    chains: Vec<Chain> [cached];
    return_chains: Vec<Chain> [cached];
    /// Length of Looper's buffer.
    loop_length: f64 [get observe];
    /// Whether Looper switches to overdub after recording a fixed number of
    /// bars; otherwise it switches to playback.
    overdub_after_record: bool [get set observe];
    /// Index of the Record Length chooser entry.
    record_length_index: i32 [get set observe];
    /// Entries of the Record Length chooser.
    record_length_list: Vec<String> [get];
    /// Tempo of Looper's buffer.
    tempo: f64 [get set observe];
}}

impl LooperDevice {
    /// View aspects of the device.
    pub fn view(&self) -> DeviceView {
        DeviceView::new(self.handle.client(), self.id())
    }

    /// Erases the recorded content.
    pub async fn clear(&self) -> Result<()> {
        self.handle.call("clear", None).await
    }

    /// Doubles the buffer length.
    pub async fn double_length(&self) -> Result<()> {
        self.handle.call("double_length", None).await
    }

    /// Doubles the playback speed.
    pub async fn double_speed(&self) -> Result<()> {
        self.handle.call("double_speed", None).await
    }

    /// Exports the content to a Session View clip slot.
    pub async fn export_to_clip_slot(&self, slot: impl IntoLiveId) -> Result<()> {
        self.handle
            .call(
                "export_to_clip_slot",
                json!({ "slot_id": slot.into_live_id() }),
            )
            .await
    }

    /// Halves the buffer length.
    pub async fn half_length(&self) -> Result<()> {
        self.handle.call("half_length", None).await
    }

    /// Halves the playback speed.
    pub async fn half_speed(&self) -> Result<()> {
        self.handle.call("half_speed", None).await
    }

    /// Plays back while adding layers of incoming audio.
    pub async fn overdub(&self) -> Result<()> {
        self.handle.call("overdub", None).await
    }

    /// Plays back without overdubbing.
    pub async fn play(&self) -> Result<()> {
        self.handle.call("play", None).await
    }

    /// Records incoming audio.
    pub async fn record(&self) -> Result<()> {
        self.handle.call("record", None).await
    }

    /// Stops playback.
    pub async fn stop(&self) -> Result<()> {
        self.handle.call("stop", None).await
    }

    /// Erases everything recorded since overdub was last enabled. Calling it
    /// again restores what the previous undo erased.
    pub async fn undo(&self) -> Result<()> {
        self.handle.call("undo", None).await
    }
}
