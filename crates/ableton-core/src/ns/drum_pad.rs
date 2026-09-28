use serde::{Deserialize, Serialize};

use super::DrumChain;
use crate::error::Result;

/// A drum pad as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawDrumPad {
    pub id: String,
    pub name: String,
    pub note: Option<i32>,
    pub mute: bool,
    pub solo: bool,
}

live_object! {
    /// A single pad of a Drum Rack. Its own device chain (e.g. its Simpler)
    /// is reachable through `chains`; a pad usually has zero or one chains.
    object DrumPad { ns: "drum-pad", raw: RawDrumPad }
}

props! { DrumPad {
    chains: Vec<DrumChain> [cached observe];
    mute: bool [get set observe];
    name: String [get observe];
    note: Option<i32> [get];
    solo: bool [get set observe];
}}

impl DrumPad {
    /// Deletes all chains of this pad, like clearing a pad in Live.
    pub async fn delete_all_chains(&self) -> Result<()> {
        self.handle.call("delete_all_chains", None).await
    }
}
