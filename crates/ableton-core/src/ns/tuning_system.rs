use serde::{Deserialize, Serialize};

/// A note index within the pseudo-octave plus an octave number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PitchClassAndOctave {
    pub index_in_octave: i32,
    pub octave: i32,
}

/// Reference pitch of a tuning system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReferencePitch {
    pub index_in_octave: i32,
    pub octave: i32,
    /// Frequency in Hz.
    pub frequency: f64,
}

crate::wire::wire_via_serde!(PitchClassAndOctave, ReferencePitch);

/// A tuning system as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RawTuningSystem {
    pub id: String,
    pub name: String,
}

live_object! {
    /// The active tuning system of the Live Set.
    object TuningSystem { ns: "tuning-system", raw: RawTuningSystem }
}

props! { TuningSystem {
    highest_note: PitchClassAndOctave [get set observe];
    lowest_note: PitchClassAndOctave [get set observe];
    name: String [get set observe];
    note_tunings: Vec<f64> [get set observe];
    number_of_notes_in_pseudo_octave: i32 [get];
    pseudo_octave_in_cents: f64 [get];
    reference_pitch: ReferencePitch [get set observe];
}}
