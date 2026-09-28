use super::AnyDevice;
use crate::error::Result;

string_enum! {
    /// Where new devices are inserted.
    pub enum DeviceInsertMode {
        Default = "default",
        SelectedLeft = "selected_left",
        SelectedRight = "selected_right",
    }
}

live_object! {
    /// View aspects of a track.
    view TrackView { ns: "track-view" }
}

props! { TrackView {
    // Live reports `device_insert_mode` as a boolean, so it's write-only here.
    device_insert_mode: DeviceInsertMode [set];
    is_collapsed: bool [get set observe];
    selected_device: Option<AnyDevice> [cached observe];
}}

impl TrackView {
    /// Selects the track's instrument, if it has one.
    pub async fn select_instrument(&self) -> Result<()> {
        self.handle.call("select_instrument", None).await
    }
}
