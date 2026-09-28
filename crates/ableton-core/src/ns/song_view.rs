use serde_json::{Value, json};

use super::{AnyChain, Clip, ClipSlot, DeviceParameter, IntoLiveId, Scene, Track};
use crate::error::Result;

live_object! {
    /// View aspects of the song: selection, detail clip, draw mode, …
    singleton SongView { ns: "song-view" }
}

props! { SongView {
    detail_clip: Option<Clip> [cached observe];
    draw_mode: bool [get set observe];
    follow_song: bool [get set observe];
    highlighted_clip_slot: Option<ClipSlot> [cached];
    selected_chain: Option<AnyChain> [get observe];
    selected_parameter: Option<DeviceParameter> [cached observe];
    selected_scene: Option<Scene> [cached observe];
    selected_track: Option<Track> [cached observe];
}}

impl SongView {
    /// Shows `clip` in Detail View.
    pub async fn set_detail_clip(&self, clip: impl IntoLiveId) -> Result<()> {
        self.handle.set("detail_clip", clip.into_live_id()).await
    }

    /// Highlights a clip slot.
    pub async fn set_highlighted_clip_slot(&self, slot: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("highlighted_clip_slot", slot.into_live_id())
            .await
    }

    /// Selects a scene.
    pub async fn set_selected_scene(&self, scene: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("selected_scene", scene.into_live_id())
            .await
    }

    /// Selects a track.
    pub async fn set_selected_track(&self, track: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("selected_track", track.into_live_id())
            .await
    }

    /// Selects a device.
    pub async fn select_device(&self, device: impl IntoLiveId) -> Result<Value> {
        self.handle
            .call(
                "select_device",
                json!({ "device_id": device.into_live_id() }),
            )
            .await
    }
}
