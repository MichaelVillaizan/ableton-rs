use serde_json::Value;

use super::{AnyChain, DrumPad, IntoLiveId};
use crate::error::Result;

live_object! {
    /// View aspects of a rack device.
    view RackDeviceView { ns: "rack-device-view" }
}

props! { RackDeviceView {
    /// Drum Racks only; `None` on other racks.
    drum_pads_scroll_position: Option<i32> [get set(i32) observe];
    is_collapsed: bool [get set observe];
    is_showing_chain_devices: bool [get set observe];
    selected_chain: Option<AnyChain> [cached observe];
    /// Drum Racks only; `None` on other racks.
    selected_drum_pad: Option<DrumPad> [cached observe];
}}

impl RackDeviceView {
    /// Selects a chain of the rack.
    pub async fn set_selected_chain(&self, chain: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("selected_chain", chain.into_live_id())
            .await
    }

    /// Clears the chain selection.
    pub async fn clear_selected_chain(&self) -> Result<()> {
        self.handle.set("selected_chain", Value::Null).await
    }

    /// Selects a drum pad of the rack.
    pub async fn set_selected_drum_pad(&self, pad: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("selected_drum_pad", pad.into_live_id())
            .await
    }
}
