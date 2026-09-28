use serde::{Deserialize, Serialize};

use crate::types::Color;

/// A rack chain as serialized by the Remote Script. Drum Rack chains carry
/// the extra drum fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawChain {
    pub id: String,
    pub name: String,
    pub color: Option<Color>,
    pub mute: bool,
    pub solo: bool,
    pub is_drum_chain: Option<bool>,
    pub choke_group: Option<i32>,
    pub in_note: Option<i32>,
    pub out_note: Option<i32>,
}

live_object! {
    /// A device chain inside a rack (Instrument, Audio Effect or MIDI Effect
    /// Rack). Not every device has chains; check
    /// [`Device::can_have_chains`](super::Device::can_have_chains) first.
    ///
    /// Drum Rack chains are [`DrumChain`](super::DrumChain)s, see
    /// [`AnyChain`](super::AnyChain).
    object Chain { ns: "chain", raw: RawChain }
}

/// Properties and functions shared by [`Chain`] and
/// [`DrumChain`](super::DrumChain).
macro_rules! chain_members {
    ($ty:ident) => {
        props! { $ty {
            color: Option<$crate::types::Color> [get set($crate::types::Color) observe];
            color_index: i32 [get set observe];
            devices: Vec<$crate::ns::AnyDevice> [cached observe];
            has_audio_input: bool [get];
            has_audio_output: bool [get];
            has_midi_input: bool [get];
            has_midi_output: bool [get];
            is_auto_colored: bool [get set observe];
            mixer_device: $crate::ns::ChainMixerDevice [cached];
            mute: bool [get set observe];
            muted_via_solo: bool [get observe];
            name: String [get set observe];
            solo: bool [get set observe];
        }}

        impl $ty {
            /// Deletes the device at `index` in this chain's `devices`.
            pub async fn delete_device(&self, index: usize) -> $crate::Result<()> {
                self.handle
                    .call("delete_device", ::serde_json::json!({ "index": index }))
                    .await
            }

            /// Duplicates the device at `index` in this chain.
            pub async fn duplicate_device(&self, index: usize) -> $crate::Result<()> {
                self.handle
                    .call("duplicate_device", ::serde_json::json!({ "index": index }))
                    .await
            }

            /// Inserts a native Live device by its UI name at `index`, or at
            /// the end when `index` is `None`. Available since Live 12.3.
            pub async fn insert_device(
                &self,
                device_name: &str,
                index: Option<usize>,
            ) -> $crate::Result<$crate::ns::AnyDevice> {
                self.handle
                    .call(
                        "insert_device",
                        ::serde_json::json!({
                            "device_name": device_name,
                            "device_index": $crate::ns::index_or_end(index),
                        }),
                    )
                    .await
            }
        }
    };
}

pub(crate) use chain_members;

chain_members!(Chain);
