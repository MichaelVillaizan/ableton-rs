use serde_json::json;

use super::device::device_members;
use super::{AnyChain, Chain, DeviceParameter, DrumPad, RackDeviceView, RawDevice, index_or_end};
use crate::error::Result;

/// Live `class_name`s of Instrument, Drum, Audio Effect and MIDI Effect Racks.
pub const RACK_CLASS_NAMES: [&str; 4] = [
    "InstrumentGroupDevice",
    "DrumGroupDevice",
    "AudioEffectGroupDevice",
    "MidiEffectGroupDevice",
];

/// Whether `class_name` belongs to a rack device.
pub fn is_rack_class_name(class_name: &str) -> bool {
    RACK_CLASS_NAMES.contains(&class_name)
}

live_object! {
    /// A rack device: Instrument, Drum, Audio Effect or MIDI Effect Rack.
    object RackDevice { ns: "rack-device", raw: RawDevice }
}

device_members!(RackDevice);

props! { RackDevice {
    /// Whether the rack can show its chains in Session View.
    can_show_chains: bool [get];
    /// Chain selector parameter; `None` on Drum Racks.
    chain_selector: Option<DeviceParameter> [cached];
    chains: Vec<AnyChain> [cached observe];
    drum_pads: Vec<DrumPad> [observe];
    /// Whether this is a Drum Rack that has drum pads.
    has_drum_pads: bool [get observe];
    /// Whether any macro is mapped to a parameter.
    has_macro_mappings: bool [get observe];
    is_showing_chains: bool [get set observe];
    /// Per macro: whether it's mapped.
    macros_mapped: Vec<bool> [get observe];
    return_chains: Vec<Chain> [cached observe];
    selected_variation_index: i32 [get set];
    variation_count: i32 [get observe];
    /// Visible drum pads; empty on non-drum racks.
    visible_drum_pads: Vec<DrumPad> [cached observe];
    visible_macro_count: i32 [get observe];
}}

impl RackDevice {
    /// View aspects of the rack.
    pub fn view(&self) -> RackDeviceView {
        RackDeviceView::new(self.handle.client(), self.id())
    }

    /// Shows one more macro control.
    pub async fn add_macro(&self) -> Result<()> {
        self.handle.call("add_macro", None).await
    }

    /// Copies all contents of a drum pad into another one. Indices are note
    /// numbers (0–127). Fails when the source pad is empty.
    pub async fn copy_pad(&self, source_index: u8, destination_index: u8) -> Result<()> {
        self.handle
            .call(
                "copy_pad",
                json!({ "source_index": source_index, "destination_index": destination_index }),
            )
            .await
    }

    /// Deletes the selected macro variation.
    pub async fn delete_selected_variation(&self) -> Result<()> {
        self.handle.call("delete_selected_variation", None).await
    }

    /// Inserts a new chain at `index`, or at the end when `index` is `None`.
    pub async fn insert_chain(&self, index: Option<usize>) -> Result<AnyChain> {
        self.handle
            .call("insert_chain", json!({ "index": index_or_end(index) }))
            .await
    }

    /// Randomizes all macros that aren't excluded from randomization.
    pub async fn randomize_macros(&self) -> Result<()> {
        self.handle.call("randomize_macros", None).await
    }

    /// Recalls the most recently recalled macro variation.
    pub async fn recall_last_used_variation(&self) -> Result<()> {
        self.handle.call("recall_last_used_variation", None).await
    }

    /// Recalls the selected macro variation.
    pub async fn recall_selected_variation(&self) -> Result<()> {
        self.handle.call("recall_selected_variation", None).await
    }

    /// Shows one macro control less.
    pub async fn remove_macro(&self) -> Result<()> {
        self.handle.call("remove_macro", None).await
    }

    /// Stores a new variation of the values of all mapped macros.
    pub async fn store_variation(&self) -> Result<()> {
        self.handle.call("store_variation", None).await
    }
}
