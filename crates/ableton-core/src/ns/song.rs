use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    AnyDevice, CuePoint, GroovePool, IntoLiveId, Scene, SongView, Track, TuningSystem, index_or_end,
};
use crate::error::Result;

string_enum! {
    /// Global clip launch quantization.
    pub enum Quantization {
        EightBars = "q_8_bars",
        FourBars = "q_4_bars",
        TwoBars = "q_2_bars",
        Bar = "q_bar",
        Half = "q_half",
        HalfTriplet = "q_half_triplet",
        Quarter = "q_quarter",
        QuarterTriplet = "q_quarter_triplet",
        Eighth = "q_eight",
        EighthTriplet = "q_eight_triplet",
        Sixteenth = "q_sixtenth",
        SixteenthTriplet = "q_sixtenth_triplet",
        ThirtySecond = "q_thirtytwoth",
        None = "q_no_q",
    }
}

string_enum! {
    /// Quantization applied while recording MIDI.
    pub enum RecordingQuantization {
        Eighth = "rec_q_eight",
        EighthEighthTriplet = "rec_q_eight_eight_triplet",
        EighthTriplet = "rec_q_eight_triplet",
        None = "rec_q_no_q",
        Quarter = "rec_q_quarter",
        Sixteenth = "rec_q_sixtenth",
        SixteenthSixteenthTriplet = "rec_q_sixtenth_sixtenth_triplet",
        SixteenthTriplet = "rec_q_sixtenth_triplet",
        ThirtySecond = "rec_q_thirtysecond",
    }
}

string_enum! {
    /// Format of [`Song::get_current_smpte_song_time`].
    pub enum TimeFormat {
        MsTime = "ms_time",
        Smpte24 = "smpte_24",
        Smpte25 = "smpte_25",
        Smpte30 = "smpte_30",
        Smpte30Drop = "smpte_30_drop",
        Smpte29 = "smpte_29",
    }
}

/// A position in SMPTE time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmpteTime {
    pub hours: i32,
    pub minutes: i32,
    pub seconds: i32,
    pub frames: i32,
}

crate::wire::wire_via_serde!(SmpteTime);

live_object! {
    /// The current Live Set: tracks, scenes, tempo, transport, …
    singleton Song { ns: "song" }
}

props! { Song {
    appointed_device: Option<AnyDevice> [get observe];
    arrangement_overdub: bool [get set observe];
    back_to_arranger: bool [get set observe];
    can_capture_midi: bool [get observe];
    can_jump_to_next_cue: bool [get observe];
    can_jump_to_prev_cue: bool [get observe];
    can_redo: bool [get];
    can_undo: bool [get];
    clip_trigger_quantization: Quantization [get set observe];
    count_in_duration: i32 [get set observe];
    cue_points: Vec<CuePoint> [cached observe];
    current_song_time: f64 [get set observe];
    /// Fires when data stored with [`Song::set_data`] changes.
    data: Value [observe];
    exclusive_arm: bool [get set observe];
    exclusive_solo: bool [get set];
    file_path: String [get];
    groove_amount: f64 [get set observe];
    is_ableton_link_enabled: bool [get set observe];
    is_ableton_link_start_stop_sync_enabled: bool [get set observe];
    is_counting_in: bool [get observe];
    is_playing: bool [get set observe];
    last_event_time: f64 [get];
    loop_enabled = "loop": bool [get set observe];
    loop_length: f64 [get set observe];
    loop_start: f64 [get set observe];
    master_track: Track [cached];
    metronome: bool [get set observe];
    midi_recording_quantization: RecordingQuantization [get set observe];
    name: String [get];
    nudge_down: bool [get set observe];
    nudge_up: bool [get set observe];
    overdub: bool [get set observe];
    punch_in: bool [get set observe];
    punch_out: bool [get set observe];
    re_enable_automation_enabled: bool [get set observe];
    record_mode: bool [get set observe];
    return_tracks: Vec<Track> [cached observe];
    root_note: i32 [get set observe];
    scale_intervals: Vec<i32> [get observe];
    scale_mode: bool [get set observe];
    scale_name: String [get set observe];
    scenes: Vec<Scene> [cached observe];
    select_on_launch: bool [get set];
    session_automation_record: bool [get set observe];
    session_record: bool [get set observe];
    session_record_status: i32 [get set observe];
    signature_denominator: i32 [get set observe];
    signature_numerator: i32 [get set observe];
    song_length: f64 [get observe];
    start_time: f64 [get set observe];
    swing_amount: f64 [get set observe];
    tempo: f64 [get set observe];
    tempo_follower_enabled: bool [get set observe];
    tracks: Vec<Track> [cached observe];
    tuning_system: Option<TuningSystem> [cached observe];
    visible_tracks: Vec<Track> [cached observe];
}}

impl Song {
    /// View aspects of the song (selection, detail clip, …).
    pub fn view(&self) -> SongView {
        SongView::new(self.handle.client())
    }

    /// The grooves of the Live Set.
    pub fn groove_pool(&self) -> GroovePool {
        GroovePool::new(self.handle.client())
    }

    /// Makes `device` the appointed device (the one "blue hand" controllers
    /// follow).
    pub async fn set_appointed_device(&self, device: impl IntoLiveId) -> Result<()> {
        self.handle
            .set("appointed_device", device.into_live_id())
            .await
    }

    /// Starts grouping the following edits into one undo step.
    pub async fn begin_undo_step(&self) -> Result<()> {
        self.handle.call("begin_undo_step", None).await
    }

    /// Continues playing from the current position.
    pub async fn continue_playing(&self) -> Result<()> {
        self.handle.call("continue_playing", None).await
    }

    /// Creates an audio track at `index`, or at the end when `None`.
    pub async fn create_audio_track(&self, index: Option<usize>) -> Result<Track> {
        self.handle
            .call(
                "create_audio_track",
                json!({ "index": index_or_end(index) }),
            )
            .await
    }

    /// Creates a MIDI track at `index`, or at the end when `None`.
    pub async fn create_midi_track(&self, index: Option<usize>) -> Result<Track> {
        self.handle
            .call("create_midi_track", json!({ "index": index_or_end(index) }))
            .await
    }

    /// Creates a return track at the end.
    pub async fn create_return_track(&self) -> Result<Track> {
        self.handle.call("create_return_track", None).await
    }

    /// Creates a scene at `index`, or at the end when `None`.
    pub async fn create_scene(&self, index: Option<usize>) -> Result<Scene> {
        self.handle
            .call("create_scene", json!({ "index": index_or_end(index) }))
            .await
    }

    /// Deletes the return track at `index`.
    pub async fn delete_return_track(&self, index: usize) -> Result<()> {
        self.handle
            .call("delete_return_track", json!([index]))
            .await
    }

    /// Deletes the scene at `index`.
    pub async fn delete_scene(&self, index: usize) -> Result<()> {
        self.handle.call("delete_scene", json!([index])).await
    }

    /// Deletes the track at `index`.
    pub async fn delete_track(&self, index: usize) -> Result<()> {
        self.handle.call("delete_track", json!([index])).await
    }

    /// Duplicates the scene at `index` and selects the copy.
    pub async fn duplicate_scene(&self, index: usize) -> Result<()> {
        self.handle.call("duplicate_scene", json!([index])).await
    }

    /// Duplicates the track at `index` and selects the copy.
    pub async fn duplicate_track(&self, index: usize) -> Result<()> {
        self.handle.call("duplicate_track", json!([index])).await
    }

    /// Ends the undo step started with [`Song::begin_undo_step`].
    pub async fn end_undo_step(&self) -> Result<()> {
        self.handle.call("end_undo_step", None).await
    }

    /// Data stored on the song with [`Song::set_data`].
    pub async fn get_data(&self, key: &str) -> Result<Value> {
        self.handle
            .call_cached("get_data", json!({ "key": key }))
            .await
    }

    /// The current playing position in the given SMPTE format.
    pub async fn get_current_smpte_song_time(&self, time_format: TimeFormat) -> Result<SmpteTime> {
        self.handle
            .call(
                "get_current_smpte_song_time",
                json!({ "timeFormat": time_format }),
            )
            .await
    }

    /// Whether the play position is on a cue point.
    pub async fn is_cue_point_selected(&self) -> Result<bool> {
        self.handle.call("is_cue_point_selected", None).await
    }

    /// Moves the play position by `amount` beats.
    pub async fn jump_by(&self, amount: f64) -> Result<()> {
        self.handle.call("jump_by", json!([amount])).await
    }

    /// Jumps to the next cue point, if possible.
    pub async fn jump_to_next_cue(&self) -> Result<()> {
        self.handle.call("jump_to_next_cue", None).await
    }

    /// Jumps to the previous cue point, if possible.
    pub async fn jump_to_prev_cue(&self) -> Result<()> {
        self.handle.call("jump_to_prev_cue", None).await
    }

    /// Plays the current selection; does nothing without one.
    pub async fn play_selection(&self) -> Result<()> {
        self.handle.call("play_selection", None).await
    }

    /// Discards overrides of automated parameters.
    pub async fn re_enable_automation(&self) -> Result<()> {
        self.handle.call("re_enable_automation", None).await
    }

    /// Redoes the last undone action.
    pub async fn redo(&self) -> Result<()> {
        self.handle.call("redo", None).await
    }

    /// Moves the play position by `amount` beats without stopping playback.
    pub async fn scrub_by(&self, amount: f64) -> Result<()> {
        self.handle.call("scrub_by", json!([amount])).await
    }

    /// Stores persistent data on the song.
    pub async fn set_data(&self, key: &str, value: impl Serialize) -> Result<()> {
        self.handle.call("set_data", json!([key, value])).await
    }

    /// Deletes the selected cue point, or creates one at the current time.
    pub async fn set_or_delete_cue(&self) -> Result<()> {
        self.handle.call("set_or_delete_cue", None).await
    }

    /// Starts playing from the start marker.
    pub async fn start_playing(&self) -> Result<()> {
        self.handle.call("start_playing", None).await
    }

    /// Stops all clips while the song keeps playing.
    pub async fn stop_all_clips(&self) -> Result<()> {
        self.handle.call("stop_all_clips", None).await
    }

    /// Stops playing.
    pub async fn stop_playing(&self) -> Result<()> {
        self.handle.call("stop_playing", None).await
    }

    /// Starts playing only when Live is stopped, so it doesn't jump back to
    /// the start. Returns whether it started.
    pub async fn safe_start_playing(&self) -> Result<bool> {
        self.handle.call("safe_start_playing", None).await
    }

    /// Stops only when Live is playing, so it doesn't jump back to the
    /// arrangement start. Returns whether it stopped.
    pub async fn safe_stop_playing(&self) -> Result<bool> {
        self.handle.call("safe_stop_playing", None).await
    }

    /// Taps the tempo.
    pub async fn tap_tempo(&self) -> Result<()> {
        self.handle.call("tap_tempo", None).await
    }

    /// Undoes the last action.
    pub async fn undo(&self) -> Result<()> {
        self.handle.call("undo", None).await
    }
}
