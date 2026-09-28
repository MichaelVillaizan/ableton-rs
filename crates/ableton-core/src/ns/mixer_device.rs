use serde::{Deserialize, Serialize};

use super::DeviceParameter;

string_enum! {
    /// Panning mode of a track.
    pub enum PanningMode {
        Stereo = "stereo",
        StereoSplit = "stereo_split",
    }
}

string_enum! {
    /// Crossfader side a track is assigned to.
    pub enum CrossfadeAssignment {
        A = "A",
        None = "NONE",
        B = "B",
    }
}

/// A track mixer as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawMixerDevice {
    pub id: String,
    /// Display string of the volume.
    pub volume: String,
}

live_object! {
    /// Mixer of a track: volume, pan, sends, crossfader, …
    object MixerDevice { ns: "mixer-device", raw: RawMixerDevice }
}

props! { MixerDevice {
    crossfade_assign: CrossfadeAssignment [get set observe];
    crossfader: DeviceParameter [get];
    cue_volume: DeviceParameter [get];
    left_split_stereo: DeviceParameter [get];
    panning: DeviceParameter [get];
    panning_mode: PanningMode [get set observe];
    right_split_stereo: DeviceParameter [get];
    sends: Vec<DeviceParameter> [get observe];
    song_tempo: DeviceParameter [get];
    track_activator: DeviceParameter [get];
    volume: DeviceParameter [get];
}}
