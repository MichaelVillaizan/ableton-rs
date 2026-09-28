//! Tauri plugin exposing Ableton Live through [`ableton_core`].
//!
//! The plugin is a thin adapter: it owns one [`Ableton`] client and
//!
//! - hands it to Rust code through [`AbletonExt::ableton`], with the full
//!   typed API of `ableton-core`;
//! - exposes the protocol-level API (`get_prop`, `set_prop`, `send_command`,
//!   listeners) to the webview as `plugin:ableton|…` commands;
//! - forwards lifecycle events to the webview as `ableton://connect`,
//!   `ableton://disconnect`, `ableton://ping` and `ableton://error`.
//!
//! ```no_run
//! # fn main() {
//! tauri::Builder::default()
//!     .plugin(tauri_plugin_ableton::init())
//!     // .run(tauri::generate_context!())
//! #   ;
//! # }
//! ```

mod commands;
mod config;
mod error;
mod state;

use ableton_core::{Ableton, AbletonOptions, ClientEvent};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime};
use tokio::sync::broadcast::error::RecvError;
use tracing::error;

pub use ableton_core;
pub use config::Config;
pub use error::{Error, Result};

use state::AbletonState;

/// Names of the events emitted to the webview.
pub mod events {
    /// Payload: `"start" | "realtime" | "heartbeat"`.
    pub const CONNECT: &str = "ableton://connect";
    /// Payload: `"realtime" | "heartbeat"`. Listeners are dropped; register
    /// them again after the next connect.
    pub const DISCONNECT: &str = "ableton://disconnect";
    /// Payload: heartbeat round-trip time in milliseconds.
    pub const PING: &str = "ableton://ping";
    /// Payload: `{ kind, message, errorType }`.
    pub const ERROR: &str = "ableton://error";
}

/// Access to the Ableton client from anything that can reach the app
/// (`AppHandle`, `App`, `Window`, …).
pub trait AbletonExt<R: Runtime> {
    /// The client managed by the plugin.
    fn ableton(&self) -> &Ableton;
}

impl<R: Runtime, T: Manager<R>> AbletonExt<R> for T {
    fn ableton(&self) -> &Ableton {
        self.state::<AbletonState>().inner().client()
    }
}

/// Initializes the plugin with the configuration from `tauri.conf.json`
/// (`plugins.ableton`).
pub fn init<R: Runtime>() -> TauriPlugin<R, Option<Config>> {
    Builder::new().build()
}

/// Configures the plugin from code.
#[derive(Debug, Default)]
pub struct Builder {
    options: Option<AbletonOptions>,
    auto_start: Option<bool>,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Client options. Takes precedence over `tauri.conf.json`.
    pub fn options(mut self, options: AbletonOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Connect to Live as soon as the app starts (default: true). Without
    /// it, call `start` from Rust or the webview.
    pub fn auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = Some(auto_start);
        self
    }

    pub fn build<R: Runtime>(self) -> TauriPlugin<R, Option<Config>> {
        tauri::plugin::Builder::<R, Option<Config>>::new("ableton")
            .invoke_handler(tauri::generate_handler![
                commands::start,
                commands::close,
                commands::is_connected,
                commands::get_ping,
                commands::get_prop,
                commands::set_prop,
                commands::send_command,
                commands::add_listener,
                commands::remove_listener,
            ])
            .setup(move |app, api| {
                let config = api.config().clone().unwrap_or_default();
                let options = self.options.unwrap_or_else(|| config.to_options());
                let auto_start = self.auto_start.or(config.auto_start).unwrap_or(true);

                let client = Ableton::new(options);
                app.manage(AbletonState::new(client.clone()));
                forward_events(app.clone(), &client);

                if auto_start {
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = client.start(None).await {
                            error!(%error, "Could not connect to Ableton Live");
                        }
                    });
                }
                Ok(())
            })
            .on_event(|app, event| {
                if let RunEvent::Exit = event {
                    if let Some(state) = app.try_state::<AbletonState>() {
                        let client = state.client().clone();
                        tauri::async_runtime::block_on(client.close());
                    }
                }
            })
            .build()
    }
}

/// Mirrors the client's lifecycle events to the webview.
fn forward_events<R: Runtime>(app: AppHandle<R>, client: &Ableton) {
    let mut events = client.events();
    tauri::async_runtime::spawn(async move {
        loop {
            let event = match events.recv().await {
                Ok(event) => event,
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return,
            };

            let emitted = match event {
                ClientEvent::Connect(kind) => app.emit(events::CONNECT, kind),
                ClientEvent::Disconnect(kind) => {
                    // The client dropped every listener; forget ours too.
                    app.state::<AbletonState>().clear_subscriptions();
                    app.emit(events::DISCONNECT, kind)
                }
                ClientEvent::Ping(latency) => {
                    app.emit(events::PING, latency.as_secs_f64() * 1000.0)
                }
                ClientEvent::Error(error) => app.emit(events::ERROR, Error::from(error)),
                // Raw traffic stays on the Rust side.
                _ => Ok(()),
            };
            if let Err(error) = emitted {
                error!(%error, "Could not emit an Ableton event");
            }
        }
    });
}
