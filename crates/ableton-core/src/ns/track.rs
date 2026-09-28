use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    AnyDevice, Clip, ClipSlot, IntoLiveId, MixerDevice, TakeLane, TrackView, index_or_end,
};
use crate::error::Result;
use crate::types::Color;

string_enum! {
    /// Channel layout of a routing channel.
    pub enum RoutingLayout {
        Midi = "midi",
        Mono = "mono",
        Stereo = "stereo",
    }
}

string_enum! {
    /// Category of a routing type.
    pub enum RoutingCategory {
        External = "external",
        Rewire = "rewire",
        Resampling = "resampling",
        Master = "master",
        Track = "track",
        ParentGroupTrack = "parent_group_track",
        None = "none",
        Invalid = "invalid",
    }
}

string_enum! {
    /// Live's `Track.monitoring_states`.
    pub enum MonitoringState {
        In = "IN",
        Auto = "AUTO",
        Off = "OFF",
    }
}

string_enum! {
    /// Kind of track.
    pub enum TrackType {
        Audio = "audio",
        Midi = "midi",
        Return = "return",
        Main = "main",
        Group = "group",
    }
}

/// An input or output channel of a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingChannel {
    pub display_name: String,
    pub layout: RoutingLayout,
}

/// An input or output type of a track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingType {
    pub display_name: String,
    pub category: RoutingCategory,
}

crate::wire::wire_via_serde!(RoutingChannel, RoutingType);

/// Selects a routing channel or type, either by its display name or exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum RoutingSelector {
    /// The first entry with this display name.
    Name(String),
    /// The entry with this display name and layout.
    Channel(RoutingChannel),
    /// The entry with this display name and category.
    Type(RoutingType),
}

impl From<&str> for RoutingSelector {
    fn from(name: &str) -> Self {
        Self::Name(name.to_owned())
    }
}

impl From<String> for RoutingSelector {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl From<RoutingChannel> for RoutingSelector {
    fn from(channel: RoutingChannel) -> Self {
        Self::Channel(channel)
    }
}

impl From<RoutingType> for RoutingSelector {
    fn from(routing_type: RoutingType) -> Self {
        Self::Type(routing_type)
    }
}

/// A track as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawTrack {
    pub id: String,
    pub name: String,
    pub color: Color,
    pub color_index: i32,
    pub is_grouped: bool,
    pub mute: bool,
    pub solo: bool,
    #[serde(rename = "type")]
    pub kind: TrackType,
}

live_object! {
    /// An audio, MIDI, group, return or main track.
    object Track { ns: "track", raw: RawTrack }
}

props! { Track {
    arm: bool [get set observe];
    arrangement_clips: Vec<Clip> [cached observe];
    available_input_routing_channels: Vec<RoutingChannel> [get observe];
    available_input_routing_types: Vec<RoutingType> [get observe];
    available_output_routing_channels: Vec<RoutingChannel> [get observe];
    available_output_routing_types: Vec<RoutingType> [get observe];
    back_to_arranger: bool [get set observe];
    can_be_armed: bool [get];
    can_be_frozen: bool [get];
    can_show_chains: bool [get];
    clip_slots: Vec<ClipSlot> [cached observe];
    color: Color [get set observe];
    color_index: i32 [get set observe];
    current_input_routing: String [get set observe];
    current_input_sub_routing: String [get set observe];
    current_monitoring_state: MonitoringState [get set observe];
    current_output_routing: String [get set observe];
    current_output_sub_routing: String [get set observe];
    devices: Vec<AnyDevice> [cached observe];
    fired_slot_index: i32 [get observe];
    fold_state: bool [get set];
    group_track: Option<Track> [cached];
    has_audio_input: bool [get observe];
    has_audio_output: bool [get observe];
    has_midi_input: bool [get observe];
    has_midi_output: bool [get observe];
    implicit_arm: bool [get set observe];
    input_meter_left: f64 [get observe];
    input_meter_level: f64 [get observe];
    input_meter_right: f64 [get observe];
    input_routing_channel: RoutingChannel [get set(RoutingSelector) observe];
    input_routing_type: RoutingType [get set(RoutingSelector) observe];
    input_routings: Vec<String> [get observe];
    input_sub_routings: Vec<String> [get observe];
    is_foldable: bool [get];
    is_frozen: bool [get observe];
    is_grouped: bool [get];
    is_part_of_selection: bool [get];
    is_showing_chains: bool [get set observe];
    is_visible: bool [get];
    mixer_device: MixerDevice [get];
    mute: bool [get set observe];
    muted_via_solo: bool [get observe];
    name: String [get set observe];
    output_meter_left: f64 [get observe];
    output_meter_level: f64 [get observe];
    output_meter_right: f64 [get observe];
    output_routing_channel: RoutingChannel [get set(RoutingSelector) observe];
    output_routing_type: RoutingType [get set(RoutingSelector) observe];
    output_routings: Vec<String> [get observe];
    output_sub_routings: Vec<String> [get observe];
    performance_impact: f64 [get observe];
    playing_slot_index: i32 [get observe];
    solo: bool [get set observe];
    take_lanes: Vec<TakeLane> [cached observe];
}}

impl Track {
    /// View aspects of the track.
    pub fn view(&self) -> TrackView {
        TrackView::new(self.handle.client(), self.id())
    }

    /// Duplicates `clip` into this track's arrangement at `time`. Fails when
    /// the clip and the track are of different types.
    pub async fn duplicate_clip_to_arrangement(
        &self,
        clip: impl IntoLiveId,
        time: f64,
    ) -> Result<Clip> {
        self.handle
            .call(
                "duplicate_clip_to_arrangement",
                json!({ "clip_id": clip.into_live_id(), "time": time }),
            )
            .await
    }

    /// Deletes `clip` from this track's arrangement. Fails when the clip
    /// belongs to another track.
    pub async fn delete_clip(&self, clip: impl IntoLiveId) -> Result<()> {
        self.handle
            .call("delete_clip", json!({ "clip_id": clip.into_live_id() }))
            .await
    }

    /// Deletes the device at `index` in this track's `devices`.
    pub async fn delete_device(&self, index: usize) -> Result<()> {
        self.handle.call("delete_device", json!([index])).await
    }

    /// Duplicates the device at `index`.
    pub async fn duplicate_device(&self, index: usize) -> Result<()> {
        self.handle
            .call("duplicate_device", json!({ "index": index }))
            .await
    }

    /// Duplicates the clip slot at `index` into the next free slot, creating
    /// a scene when needed. Returns the index of the new slot.
    pub async fn duplicate_clip_slot(&self, index: usize) -> Result<usize> {
        self.handle
            .call("duplicate_clip_slot", json!({ "index": index }))
            .await
    }

    /// Creates a take lane (Arrangement View comping).
    pub async fn create_take_lane(&self) -> Result<TakeLane> {
        self.handle.call("create_take_lane", None).await
    }

    /// Inserts a native Live device by its UI name at `index`, or at the end
    /// when `index` is `None`. Available since Live 12.3.
    pub async fn insert_device(
        &self,
        device_name: &str,
        index: Option<usize>,
    ) -> Result<AnyDevice> {
        self.handle
            .call(
                "insert_device",
                json!({ "device_name": device_name, "target_index": index_or_end(index) }),
            )
            .await
    }

    /// Jumps by `beats` in the running Session View clip.
    pub async fn jump_in_running_session_clip(&self, beats: f64) -> Result<()> {
        self.handle
            .call("jump_in_running_session_clip", json!({ "beats": beats }))
            .await
    }

    /// Stops all fired clips of this track.
    pub async fn stop_all_clips(&self, quantized: bool) -> Result<()> {
        self.handle
            .call("stop_all_clips", json!({ "quantized": quantized }))
            .await
    }

    /// Data stored on this track with [`Track::set_data`].
    pub async fn get_data(&self, key: &str) -> Result<Value> {
        self.handle
            .call_cached("get_data", json!({ "key": key }))
            .await
    }

    /// Stores persistent data on this track.
    pub async fn set_data(&self, key: &str, value: impl Serialize) -> Result<()> {
        self.handle
            .call("set_data", json!({ "key": key, "value": value }))
            .await
    }

    /// Creates an audio clip referencing `file_path` in the arrangement at
    /// `position`. Audio tracks only.
    pub async fn create_audio_clip(&self, file_path: &str, position: f64) -> Result<Clip> {
        self.handle
            .call(
                "create_audio_clip",
                json!({ "file_path": file_path, "position": position }),
            )
            .await
    }

    /// Creates an empty MIDI clip in the arrangement at `start_time`
    /// (0‥1576800). MIDI tracks only; fails on frozen or recording tracks.
    /// Available since Live 12.2.
    pub async fn create_midi_clip(&self, start_time: f64, length: f64) -> Result<Clip> {
        self.handle
            .call(
                "create_midi_clip",
                json!({ "start_time": start_time, "length": length }),
            )
            .await
    }
}
