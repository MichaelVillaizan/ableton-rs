use serde::{Deserialize, Serialize};

/// A MIDI note as `[pitch, time, duration, velocity, muted]`, the compact
/// form used on the wire.
pub type NoteTuple = (i32, f64, f64, f64, bool);

/// A MIDI note in a clip.
///
/// Travels as a [`NoteTuple`] over the wire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(from = "NoteTuple", into = "NoteTuple")]
pub struct Note {
    pub pitch: i32,
    pub time: f64,
    pub duration: f64,
    pub velocity: f64,
    pub muted: bool,
}

impl From<NoteTuple> for Note {
    fn from((pitch, time, duration, velocity, muted): NoteTuple) -> Self {
        Self {
            pitch,
            time,
            duration,
            velocity,
            muted,
        }
    }
}

impl From<Note> for NoteTuple {
    fn from(note: Note) -> Self {
        (
            note.pitch,
            note.time,
            note.duration,
            note.velocity,
            note.muted,
        )
    }
}

/// A MIDI note with the extended properties introduced in Live 11.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NoteExtended {
    pub note_id: i64,
    pub duration: f64,
    pub mute: bool,
    pub pitch: i32,
    pub probability: f64,
    pub release_velocity: f64,
    pub start_time: f64,
    pub velocity: f64,
    pub velocity_deviation: f64,
}

crate::wire::wire_via_serde!(Note, NoteExtended);

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn travels_as_a_tuple() {
        let note = Note {
            pitch: 60,
            time: 1.0,
            duration: 0.5,
            velocity: 100.0,
            muted: false,
        };
        let wire = json!([60, 1.0, 0.5, 100.0, false]);
        assert_eq!(serde_json::to_value(note).unwrap(), wire);
        assert_eq!(serde_json::from_value::<Note>(wire).unwrap(), note);
    }
}
