use serde_json::json;

use super::{BrowserItem, IntoLiveId};
use crate::error::Result;

string_enum! {
    /// Live's `Browser.FilterType`.
    pub enum BrowserFilterType {
        Disabled = "disabled",
        HotswapOff = "hotswap_off",
        InstrumentHotswap = "instrument_hotswap",
        AudioEffectHotswap = "audio_effect_hotswap",
        MidiEffectHotswap = "midi_effect_hotswap",
        DrumPadHotswap = "drum_pad_hotswap",
        MidiTrackDevices = "midi_track_devices",
        Samples = "samples",
        Count = "count",
    }
}

string_enum! {
    /// Live's `Browser.Relation`.
    pub enum BrowserRelation {
        Ancestor = "ancestor",
        Equal = "equal",
        Descendant = "descendant",
        None = "none",
    }
}

live_object! {
    /// Live's browser.
    singleton Browser { ns: "browser" }
}

props! { Browser {
    audio_effects: Vec<BrowserItem> [cached];
    clips: Vec<BrowserItem> [cached];
    colors: Vec<BrowserItem> [cached];
    current_project: Vec<BrowserItem> [cached];
    drums: Vec<BrowserItem> [cached];
    filter_type: BrowserFilterType [get set observe];
    /// Fires when the browser finished a full refresh.
    full_refresh: () [observe];
    /// The remote script stalls while hot-swapping, so listeners only fire
    /// when hot-swap is deactivated.
    hotswap_target: BrowserItem [cached observe];
    instruments: Vec<BrowserItem> [cached];
    legacy_libraries: Vec<BrowserItem> [cached];
    max_for_live: Vec<BrowserItem> [cached];
    midi_effects: Vec<BrowserItem> [cached];
    packs: Vec<BrowserItem> [cached];
    plugins: Vec<BrowserItem> [cached];
    samples: Vec<BrowserItem> [cached];
    sounds: Vec<BrowserItem> [cached];
    user_folders: Vec<BrowserItem> [cached];
    user_library: Vec<BrowserItem> [cached];
}}

impl Browser {
    /// Loads a browser item.
    pub async fn load_item(&self, item: impl IntoLiveId) -> Result<()> {
        self.handle
            .call("load_item", json!({ "id": item.into_live_id() }))
            .await
    }

    /// Previews a browser item.
    pub async fn preview_item(&self, item: impl IntoLiveId) -> Result<()> {
        self.handle
            .call("preview_item", json!({ "id": item.into_live_id() }))
            .await
    }

    /// How `item` relates to the current hot-swap target.
    pub async fn relation_to_hotswap_target(
        &self,
        item: impl IntoLiveId,
    ) -> Result<BrowserRelation> {
        self.handle
            .call(
                "relation_to_hotswap_target",
                json!({ "id": item.into_live_id() }),
            )
            .await
    }

    /// Stops the current preview.
    pub async fn stop_preview(&self) -> Result<()> {
        self.handle.call("stop_preview", None).await
    }
}
