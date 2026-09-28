use serde_json::json;

use crate::error::Result;

live_object! {
    /// The Session View highlight ("red box").
    singleton Session { ns: "session" }
}

impl Session {
    /// Creates the highlight with the given size.
    pub async fn setup_session_box(&self, num_tracks: usize, num_scenes: usize) -> Result<()> {
        self.handle
            .call(
                "setup_session_box",
                json!({ "num_tracks": num_tracks, "num_scenes": num_scenes }),
            )
            .await
    }

    /// Moves the highlight to the given track and scene offsets.
    pub async fn set_session_offset(&self, track_offset: usize, scene_offset: usize) -> Result<()> {
        self.handle
            .call(
                "set_session_offset",
                json!({ "track_offset": track_offset, "scene_offset": scene_offset }),
            )
            .await
    }
}
