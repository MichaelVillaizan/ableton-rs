use serde_json::Value;

use super::chain::chain_members;
use super::{Chain, Handle, LiveObject, RawChain};
use crate::Ableton;
use crate::error::Result;
use crate::wire::{FromWire, from_serde};

/// Drum chains are serialized like regular chains, with the drum fields set.
pub type RawDrumChain = RawChain;

live_object! {
    /// A chain inside a Drum Rack, with note and choke-group controls on top
    /// of a regular [`Chain`].
    object DrumChain { ns: "drum-chain", raw: RawDrumChain }
}

chain_members!(DrumChain);

props! { DrumChain {
    /// Choke group index of this chain.
    choke_group: i32 [get set observe];
    /// MIDI note that triggers this chain.
    in_note: i32 [get set observe];
    /// MIDI note this chain outputs.
    out_note: i32 [get set observe];
}}

/// Any kind of rack chain.
#[derive(Debug, Clone)]
pub enum AnyChain {
    Chain(Chain),
    Drum(DrumChain),
}

impl AnyChain {
    /// Wraps a serialized chain as a [`DrumChain`] when it carries drum
    /// fields, otherwise as a [`Chain`].
    pub fn new(client: &Ableton, raw: RawChain) -> Self {
        if raw.is_drum_chain == Some(true) {
            Self::Drum(DrumChain::new(client, raw))
        } else {
            Self::Chain(Chain::new(client, raw))
        }
    }

    /// Id of the Live object.
    pub fn id(&self) -> &str {
        self.raw().id.as_str()
    }

    /// The chain as Live serialized it.
    pub fn raw(&self) -> &RawChain {
        match self {
            Self::Chain(chain) => chain.raw(),
            Self::Drum(chain) => chain.raw(),
        }
    }

    /// Whether this is a Drum Rack chain.
    pub fn is_drum_chain(&self) -> bool {
        matches!(self, Self::Drum(_))
    }

    /// The chain as a [`DrumChain`], if it is one.
    pub fn as_drum_chain(&self) -> Option<&DrumChain> {
        match self {
            Self::Drum(chain) => Some(chain),
            Self::Chain(_) => None,
        }
    }

    /// The chain as a plain [`Chain`], giving access to the properties every
    /// chain has.
    pub fn as_chain(&self) -> Chain {
        match self {
            Self::Chain(chain) => chain.clone(),
            Self::Drum(chain) => Chain::new(chain.handle().client(), chain.raw().clone()),
        }
    }
}

impl LiveObject for AnyChain {
    fn handle(&self) -> &Handle {
        match self {
            Self::Chain(chain) => chain.handle(),
            Self::Drum(chain) => chain.handle(),
        }
    }
}

impl FromWire for AnyChain {
    fn from_wire(client: &Ableton, value: Value) -> Result<Self> {
        Ok(Self::new(client, from_serde(value)?))
    }
}
