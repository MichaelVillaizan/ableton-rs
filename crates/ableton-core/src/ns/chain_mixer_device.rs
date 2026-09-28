use serde::{Deserialize, Serialize};

use super::DeviceParameter;

/// A chain mixer as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawChainMixerDevice {
    pub id: String,
    /// Display string of the volume; `None` on MIDI Effect Rack chains.
    pub volume: Option<String>,
}

live_object! {
    /// Mixer of a rack chain: chain activator, volume, pan and sends.
    object ChainMixerDevice { ns: "chain-mixer-device", raw: RawChainMixerDevice }
}

props! { ChainMixerDevice {
    chain_activator: DeviceParameter [get];
    /// `None` on MIDI Effect Rack chains.
    panning: Option<DeviceParameter> [get];
    sends: Vec<DeviceParameter> [get observe];
    /// `None` on MIDI Effect Rack chains.
    volume: Option<DeviceParameter> [get];
}}
