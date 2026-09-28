use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Envelope, IntoLiveId};
use crate::error::Result;
use crate::types::{Color, Note, NoteExtended};

string_enum! {
    /// Warp mode of an audio clip.
    pub enum WarpMode {
        Beats = "beats",
        Tones = "tones",
        Texture = "texture",
        Repitch = "repitch",
        Complex = "complex",
        Rex = "rex",
        ComplexPro = "complex_pro",
    }
}

string_enum! {
    /// How a clip reacts to its launch button.
    pub enum LaunchMode {
        Trigger = "trigger",
        Gate = "gate",
        Toggle = "toggle",
        Repeat = "repeat",
    }
}

string_enum! {
    /// Launch quantization of a clip.
    pub enum LaunchQuantization {
        Global = "q_global",
        None = "q_none",
        EightBars = "q_8_bars",
        FourBars = "q_4_bars",
        TwoBars = "q_2_bars",
        Bar = "q_bar",
        Half = "q_half",
        HalfTriplet = "q_half_triplet",
        Quarter = "q_quarter",
        QuarterTriplet = "q_quarter_triplet",
        Eighth = "q_eighth",
        EighthTriplet = "q_eighth_triplet",
        Sixteenth = "q_sixteenth",
        SixteenthTriplet = "q_sixteenth_triplet",
        ThirtySecond = "q_thirtysecond",
    }
}

/// A warp marker of an audio clip.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WarpMarker {
    pub beat_time: f64,
    pub sample_time: f64,
}

crate::wire::wire_via_serde!(WarpMarker);

/// A clip as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawClip {
    pub id: String,
    pub name: String,
    pub color: Color,
    pub color_index: i32,
    pub is_audio_clip: bool,
    pub is_midi_clip: bool,
    pub start_time: f64,
    pub end_time: f64,
    pub muted: bool,
}

live_object! {
    /// A clip in Session View or in the arrangement.
    object Clip { ns: "clip", raw: RawClip }
}

props! { Clip {
    available_warp_modes: Vec<WarpMode> [get];
    color: Color [get set observe];
    color_index: i32 [get set observe];
    end_marker: f64 [get set observe];
    end_time: f64 [get observe];
    file_path: String [get observe];
    gain: f64 [get set observe];
    gain_display_string: String [get];
    has_envelopes: bool [get observe];
    is_arrangement_clip: bool [get];
    is_audio_clip: bool [get];
    is_midi_clip: bool [get];
    is_overdubbing: bool [get observe];
    is_playing: bool [get set];
    is_recording: bool [get observe];
    is_triggered: bool [get];
    launch_mode: LaunchMode [get set];
    launch_quantization: LaunchQuantization [get set];
    length: f64 [get];
    loop_end: f64 [get set observe];
    loop_start: f64 [get set observe];
    looping: bool [get set];
    muted: bool [get set observe];
    name: String [get set observe];
    notes: Vec<Note> [observe];
    pitch_coarse: i32 [get set observe];
    pitch_fine: f64 [get set observe];
    playing_position: f64 [get observe];
    position: f64 [get set observe];
    ram_mode: bool [get set];
    sample_length: f64 [get];
    signature_denominator: i32 [get set observe];
    signature_numerator: i32 [get set observe];
    start_marker: f64 [get set observe];
    start_time: f64 [get];
    velocity_amount: f64 [get set];
    warp_mode: WarpMode [get set];
    /// Only supported in Live 11.
    warp_markers: Vec<WarpMarker> [get observe(Value)];
    warping: bool [get set observe];
    will_record_on_start: bool [get];
}}

impl Clip {
    /// Audio clips only: converts beat time to sample time. Fails when the
    /// sample isn't warped.
    pub async fn beat_to_sample_time(&self, beats: f64) -> Result<f64> {
        self.handle
            .call("beat_to_sample_time", json!([beats]))
            .await
    }

    /// Clears all envelopes of this clip.
    pub async fn clear_all_envelopes(&self) -> Result<()> {
        self.handle.call("clear_all_envelopes", None).await
    }

    /// The envelope of the given parameter, if there is one. Arrangement
    /// clips and parameters of other tracks always return `None`.
    pub async fn automation_envelope(
        &self,
        parameter: impl IntoLiveId,
    ) -> Result<Option<Envelope>> {
        self.handle
            .call(
                "automation_envelope",
                json!({ "parameter_id": parameter.into_live_id() }),
            )
            .await
    }

    /// Clears the envelope of the given parameter.
    pub async fn clear_envelope(&self, parameter: impl IntoLiveId) -> Result<()> {
        self.handle
            .call(
                "clear_envelope",
                json!({ "parameter_id": parameter.into_live_id() }),
            )
            .await
    }

    /// Creates an envelope for the given parameter. Only use it when the
    /// envelope doesn't exist yet; fails when it can't be created.
    pub async fn create_automation_envelope(&self, parameter: impl IntoLiveId) -> Result<Envelope> {
        self.handle
            .call(
                "create_automation_envelope",
                json!({ "parameter_id": parameter.into_live_id() }),
            )
            .await
    }

    /// Crops the clip: removes what's outside the loop when it's looped, or
    /// outside the start and end markers otherwise.
    pub async fn crop(&self) -> Result<()> {
        self.handle.call("crop", None).await
    }

    /// Deselects all notes.
    pub async fn deselect_all_notes(&self) -> Result<()> {
        self.handle.call("deselect_all_notes", None).await
    }

    /// Doubles the loop and duplicates its notes and envelopes. Duplicates
    /// the start/end range when the clip isn't looped.
    pub async fn duplicate_loop(&self) -> Result<()> {
        self.handle.call("duplicate_loop", None).await
    }

    /// Duplicates the notes of a region to `destination_time`, optionally
    /// only those of one `pitch`, transposed by `transposition_amount`
    /// semitones. Fails on audio clips.
    pub async fn duplicate_region(
        &self,
        start: f64,
        length: f64,
        destination_time: f64,
        pitch: Option<u8>,
        transposition_amount: i32,
    ) -> Result<()> {
        let pitch = pitch.map_or(-1, i32::from);
        self.handle
            .call(
                "duplicate_region",
                json!([start, length, destination_time, pitch, transposition_amount]),
            )
            .await
    }

    /// Starts playing the clip.
    pub async fn fire(&self) -> Result<()> {
        self.handle.call("fire", None).await
    }

    /// Notes in the given time and pitch range.
    pub async fn get_notes(
        &self,
        from_time: f64,
        from_pitch: i32,
        time_span: f64,
        pitch_span: i32,
    ) -> Result<Vec<Note>> {
        self.handle
            .call(
                "get_notes",
                json!({
                    "from_time": from_time,
                    "from_pitch": from_pitch,
                    "time_span": time_span,
                    "pitch_span": pitch_span,
                }),
            )
            .await
    }

    /// Notes in the given range, with their extended properties.
    pub async fn get_notes_extended(
        &self,
        from_time: f64,
        from_pitch: i32,
        time_span: f64,
        pitch_span: i32,
    ) -> Result<Vec<NoteExtended>> {
        self.handle
            .call(
                "get_notes_extended",
                json!({
                    "from_pitch": from_pitch,
                    "pitch_span": pitch_span,
                    "from_time": from_time,
                    "time_span": time_span,
                }),
            )
            .await
    }

    /// The selected notes.
    pub async fn get_selected_notes(&self) -> Result<Vec<Note>> {
        self.handle.call("get_selected_notes", None).await
    }

    /// The selected notes, with their extended properties.
    pub async fn get_selected_notes_extended(&self) -> Result<Vec<NoteExtended>> {
        self.handle.call("get_selected_notes_extended", None).await
    }

    /// Modifies existing notes in place. Available since Live 11.
    pub async fn apply_note_modifications(&self, notes: &[NoteExtended]) -> Result<()> {
        self.handle
            .call("apply_note_modifications", json!({ "notes": notes }))
            .await
    }

    /// Jumps by `amount` beats. Does nothing when the clip isn't playing.
    pub async fn move_playing_pos(&self, amount: f64) -> Result<()> {
        self.handle.call("move_playing_pos", json!([amount])).await
    }

    /// Quantizes all notes, or aligns warp markers.
    pub async fn quantize(&self, grid: i32, amount: f64) -> Result<()> {
        self.handle.call("quantize", json!([grid, amount])).await
    }

    /// Quantizes all notes of one pitch.
    pub async fn quantize_pitch(&self, pitch: i32, grid: i32, amount: f64) -> Result<()> {
        self.handle
            .call("quantize_pitch", json!([pitch, grid, amount]))
            .await
    }

    /// Deletes all notes starting in the given area.
    #[deprecated = "starting with Live 11, use `remove_notes_extended` instead"]
    pub async fn remove_notes(
        &self,
        from_time: f64,
        from_pitch: i32,
        time_span: f64,
        pitch_span: i32,
    ) -> Result<()> {
        self.handle
            .call(
                "remove_notes",
                json!([from_time, from_pitch, time_span, pitch_span]),
            )
            .await
    }

    /// Deletes all notes starting in the given area.
    pub async fn remove_notes_extended(
        &self,
        from_time: f64,
        from_pitch: i32,
        time_span: f64,
        pitch_span: i32,
    ) -> Result<()> {
        self.handle
            .call(
                "remove_notes_extended",
                json!([from_pitch, pitch_span, from_time, time_span]),
            )
            .await
    }

    /// Removes notes by id. Available since Live 11.
    pub async fn remove_notes_by_id(&self, ids: &[i64]) -> Result<()> {
        self.handle.call("remove_notes_by_id", json!([ids])).await
    }

    /// Replaces the selected notes.
    pub async fn replace_selected_notes(&self, notes: &[Note]) -> Result<()> {
        self.handle
            .call("replace_selected_notes", json!({ "notes": notes }))
            .await
    }

    /// Audio clips only: converts sample time to beat time. Fails when the
    /// sample isn't warped.
    pub async fn sample_to_beat_time(&self, sample_time: f64) -> Result<f64> {
        self.handle
            .call("sample_to_beat_time", json!([sample_time]))
            .await
    }

    /// Starts scrubbing at `position` (in beats) until [`Clip::stop_scrub`].
    /// Global quantization applies.
    pub async fn scrub(&self, position: f64) -> Result<()> {
        self.handle.call("scrub", json!([position])).await
    }

    /// Audio clips only: converts seconds to sample time. Fails when the
    /// sample is warped.
    pub async fn seconds_to_sample_time(&self, seconds: f64) -> Result<f64> {
        self.handle
            .call("seconds_to_sample_time", json!([seconds]))
            .await
    }

    /// Selects all notes.
    pub async fn select_all_notes(&self) -> Result<()> {
        self.handle.call("select_all_notes", None).await
    }

    /// Sets the state of the clip's launch button directly. Supports all
    /// launch modes.
    pub async fn set_fire_button_state(&self, state: bool) -> Result<()> {
        self.handle
            .call("set_fire_button_state", json!([state]))
            .await
    }

    /// Adds notes to the clip.
    pub async fn set_notes(&self, notes: &[Note]) -> Result<()> {
        self.handle
            .call("set_notes", json!({ "notes": notes }))
            .await
    }

    /// Stops playing the clip.
    pub async fn stop(&self) -> Result<()> {
        self.handle.call("stop", None).await
    }

    /// Stops the current scrub.
    pub async fn stop_scrub(&self) -> Result<()> {
        self.handle.call("stop_scrub", None).await
    }
}
