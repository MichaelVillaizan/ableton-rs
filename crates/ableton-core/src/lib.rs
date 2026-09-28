//! Control Ableton Live from Rust.
//!
//! This crate talks to the AbletonJS MIDI Remote Script (the `midi-script`
//! folder of `ableton-js`) over the same WebSocket protocol as the JS client,
//! and exposes the same object model: [`Ableton`] is the client, and
//! [`Ableton::song`], [`Ableton::application`], … hand out typed Live objects
//! whose properties are plain async methods.
//!
//! ```no_run
//! use ableton_core::prelude::*;
//!
//! # async fn run() -> ableton_core::Result<()> {
//! let ableton = Ableton::new(AbletonOptions::default());
//! ableton.start(None).await?;
//!
//! let song = ableton.song();
//! println!("tempo: {}", song.tempo().await?);
//! song.set_tempo(128.0).await?;
//!
//! let sub = song
//!     .on_is_playing(|playing| println!("playing: {playing}"))
//!     .await?;
//!
//! for track in song.tracks().await? {
//!     println!("{} ({})", track.raw().name, track.raw().kind);
//! }
//!
//! sub.unsubscribe().await?;
//! ableton.close().await;
//! # Ok(())
//! # }
//! ```
//!
//! ## Layout
//!
//! - [`client`]: connection lifecycle, command batching, caching and
//!   property listeners.
//! - [`ns`]: the Live object model (`Song`, `Track`, `Clip`, …).
//! - [`wire`]: how JSON values coming from Live become typed Rust values.
//! - [`types`]: value types shared between namespaces (colors, notes).

#[macro_use]
mod macros;

mod auth;
mod cache;
pub mod client;
pub mod error;
pub mod events;
pub mod ns;
pub mod options;
pub mod protocol;
pub mod types;
pub mod wire;

pub use client::Ableton;
pub use error::{Error, Result};
pub use events::{ClientEvent, ConnectKind, DisconnectKind};
pub use options::{AbletonOptions, CacheOptions};

/// Version of the AbletonJS Remote Script this crate speaks to.
///
/// On start the client compares it with the version reported by the plugin
/// and logs a warning when they differ.
pub const PLUGIN_VERSION: &str = "5.0.3";

/// Everything needed for day-to-day use.
pub mod prelude {
    pub use crate::client::Ableton;
    pub use crate::error::{Error, Result};
    pub use crate::events::{ClientEvent, ConnectKind, DisconnectKind};
    pub use crate::ns::*;
    pub use crate::options::{AbletonOptions, CacheOptions};
    pub use crate::types::*;
}
