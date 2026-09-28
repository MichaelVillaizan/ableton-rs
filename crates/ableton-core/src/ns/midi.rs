use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Ableton;
use crate::error::{Error, Result};
use crate::wire::{FromWire, from_serde};

/// Status byte of a MIDI message, with the values the JS client uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MidiCommand {
    NoteOn = 128,
    NoteOff = 144,
    AfterTouch = 160,
    ControlChange = 176,
    PatchChange = 192,
    ChannelPressure = 208,
    PitchBend = 224,
    SysExStart = 240,
    MidiTimeCodeQuarterFrame = 241,
    SongPositionPointer = 242,
    SongSelect = 243,
    TuneRequest = 246,
    SysExEnd = 247,
    TimingClock = 248,
    Start = 250,
    Continue = 251,
    Stop = 252,
    ActiveSensing = 254,
    SystemReset = 255,
}

impl TryFrom<u8> for MidiCommand {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        use MidiCommand::*;
        Ok(match byte {
            128 => NoteOn,
            144 => NoteOff,
            160 => AfterTouch,
            176 => ControlChange,
            192 => PatchChange,
            208 => ChannelPressure,
            224 => PitchBend,
            240 => SysExStart,
            241 => MidiTimeCodeQuarterFrame,
            242 => SongPositionPointer,
            243 => SongSelect,
            246 => TuneRequest,
            247 => SysExEnd,
            248 => TimingClock,
            250 => Start,
            251 => Continue,
            252 => Stop,
            254 => ActiveSensing,
            255 => SystemReset,
            other => return Err(Error::Decode(format!("invalid midi command: {other}"))),
        })
    }
}

string_enum! {
    /// Kind of MIDI mapping forwarded by the Remote Script.
    pub enum MidiMappingType {
        Cc = "cc",
        Note = "note",
    }
}

/// A MIDI note or CC the Remote Script should forward to the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MidiMapping {
    #[serde(rename = "type")]
    pub kind: MidiMappingType,
    pub channel: u8,
    pub target: u8,
}

/// A note on/off message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiNote {
    pub command: MidiCommand,
    pub key: u8,
    pub velocity: u8,
}

/// A control change message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiCc {
    pub controller: u8,
    pub value: u8,
}

/// A MIDI message as serialized by the Remote Script.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMidiMessage {
    pub bytes: Vec<u8>,
}

/// A MIDI message received from Live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiMessage {
    pub command: MidiCommand,
    pub parameter1: Option<u8>,
    pub parameter2: Option<u8>,
}

impl TryFrom<RawMidiMessage> for MidiMessage {
    type Error = Error;

    fn try_from(raw: RawMidiMessage) -> Result<Self> {
        let (parameter1, parameter2) = match raw.bytes.as_slice() {
            [] => return Err(Error::Decode("bytes missing from midi message".into())),
            [_] => (None, None),
            [_, p1] => (Some(*p1), None),
            [_, p1, p2] => (Some(*p1), Some(*p2)),
            bytes => {
                return Err(Error::Decode(format!(
                    "invalid midi message length: {}",
                    bytes.len()
                )));
            }
        };
        Ok(Self {
            command: MidiCommand::try_from(raw.bytes[0])?,
            parameter1,
            parameter2,
        })
    }
}

impl MidiMessage {
    /// The message as a control change, or an error when it isn't one.
    pub fn to_cc(&self) -> Result<MidiCc> {
        if self.command != MidiCommand::ControlChange {
            return Err(Error::InvalidArgument("not a midi CC message".into()));
        }
        Ok(MidiCc {
            controller: self.parameter1.unwrap_or_default(),
            value: self.parameter2.unwrap_or_default(),
        })
    }

    /// The message as a note on/off, or an error when it isn't one.
    pub fn to_note(&self) -> Result<MidiNote> {
        if !matches!(self.command, MidiCommand::NoteOn | MidiCommand::NoteOff) {
            return Err(Error::InvalidArgument("not a midi note message".into()));
        }
        Ok(MidiNote {
            command: self.command,
            key: self.parameter1.unwrap_or_default(),
            velocity: self.parameter2.unwrap_or_default(),
        })
    }
}

impl FromWire for MidiMessage {
    fn from_wire(_: &Ableton, value: Value) -> Result<Self> {
        Self::try_from(from_serde::<RawMidiMessage>(value)?)
    }
}

live_object! {
    /// MIDI forwarding: tell the Remote Script which notes and CCs to send.
    singleton Midi { ns: "midi" }
}

props! { Midi {
    midi: MidiMessage [observe];
    midi_outputs: Vec<MidiMapping> [set];
}}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_messages() {
        let message = MidiMessage::try_from(RawMidiMessage {
            bytes: vec![176, 7, 100],
        })
        .unwrap();
        assert_eq!(
            message.to_cc().unwrap(),
            MidiCc {
                controller: 7,
                value: 100
            }
        );
        assert!(message.to_note().is_err());
        assert!(MidiMessage::try_from(RawMidiMessage { bytes: vec![] }).is_err());
        assert!(MidiMessage::try_from(RawMidiMessage { bytes: vec![1] }).is_err());
    }
}
