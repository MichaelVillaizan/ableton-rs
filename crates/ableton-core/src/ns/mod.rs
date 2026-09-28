//! The Live object model.
//!
//! Every type here is a lightweight handle (a client plus an object id) and
//! is cheap to clone. Property reads, writes and listeners are generated
//! methods (`tempo()`, `set_tempo()`, `on_tempo()`); anything not covered is
//! reachable through [`Handle`].

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use crate::client::Ableton;
use crate::error::Result;
use crate::protocol::Command;
use crate::wire::{FromWire, to_value};

mod application;
mod application_view;
mod browser;
mod browser_item;
mod chain;
mod chain_mixer_device;
mod clip;
mod clip_slot;
mod cue_point;
mod device;
mod device_parameter;
mod device_view;
mod drum_chain;
mod drum_pad;
mod envelope;
mod groove;
mod groove_pool;
mod internal;
mod looper_device;
mod midi;
mod mixer_device;
mod plugin_device;
mod rack_device;
mod rack_device_view;
mod scene;
mod session;
mod song;
mod song_view;
mod take_lane;
mod track;
mod track_view;
mod tuning_system;

pub use application::*;
pub use application_view::*;
pub use browser::*;
pub use browser_item::*;
pub use chain::*;
pub use chain_mixer_device::*;
pub use clip::*;
pub use clip_slot::*;
pub use cue_point::*;
pub use device::*;
pub use device_parameter::*;
pub use device_view::*;
pub use drum_chain::*;
pub use drum_pad::*;
pub use envelope::*;
pub use groove::*;
pub use groove_pool::*;
pub use internal::*;
pub use looper_device::*;
pub use midi::*;
pub use mixer_device::*;
pub use plugin_device::*;
pub use rack_device::*;
pub use rack_device_view::*;
pub use scene::*;
pub use session::*;
pub use song::*;
pub use song_view::*;
pub use take_lane::*;
pub use track::*;
pub use track_view::*;
pub use tuning_system::*;

pub use crate::client::Subscription;

/// Address of a Live object plus the client to reach it.
#[derive(Clone)]
pub struct Handle {
    client: Ableton,
    ns: &'static str,
    nsid: Option<Arc<str>>,
}

impl Handle {
    pub(crate) fn new(client: &Ableton, ns: &'static str, nsid: Option<String>) -> Self {
        Self {
            client: client.clone(),
            ns,
            nsid: nsid.map(Arc::from),
        }
    }

    /// The client this object talks through.
    pub fn client(&self) -> &Ableton {
        &self.client
    }

    /// Namespace of the object in the Remote Script.
    pub fn ns(&self) -> &'static str {
        self.ns
    }

    /// Id of the object, if it has one.
    pub fn nsid(&self) -> Option<&str> {
        self.nsid.as_deref()
    }

    /// Reads any property, converting it to `T`. `cache` uses the etag cache.
    pub async fn get<T: FromWire>(&self, prop: &str, cache: bool) -> Result<T> {
        let value = self
            .client
            .get_prop(self.ns, self.nsid(), prop, cache)
            .await?;
        T::from_wire(&self.client, value)
    }

    /// Sets any property.
    pub async fn set<V: Serialize>(&self, prop: &str, value: V) -> Result<()> {
        self.client
            .set_prop(self.ns, self.nsid(), prop, to_value(value)?)
            .await
            .map(drop)
    }

    /// Observes any property, converting each update to `T`. Updates that
    /// can't be converted are logged and skipped.
    pub async fn observe<T, F>(&self, prop: &str, listener: F) -> Result<Subscription>
    where
        T: FromWire,
        F: Fn(T) + Send + Sync + 'static,
    {
        // A weak handle: the client stores the listener, so a strong one
        // would keep it alive forever.
        let client = self.client.downgrade();
        let prop_name = prop.to_owned();
        self.client
            .add_prop_listener(self.ns, self.nsid(), prop, move |value| {
                let Some(client) = client.upgrade() else {
                    return;
                };
                match T::from_wire(&client, value) {
                    Ok(value) => listener(value),
                    Err(error) => warn!(prop = %prop_name, %error, "Skipped a listener update"),
                }
            })
            .await
    }

    /// Calls a function on the Live object. `args` is an object for keyword
    /// arguments or an array for positional ones.
    pub async fn call<T: FromWire>(&self, name: &str, args: impl Into<Option<Value>>) -> Result<T> {
        let value = self
            .client
            .send_command(self.command(name, args.into()))
            .await?;
        T::from_wire(&self.client, value)
    }

    /// Like [`Handle::call`], with a custom reply timeout.
    pub async fn call_with_timeout<T: FromWire>(
        &self,
        name: &str,
        args: impl Into<Option<Value>>,
        timeout: Duration,
    ) -> Result<T> {
        let command = self.command(name, args.into()).timeout(timeout);
        let value = self.client.send_command(command).await?;
        T::from_wire(&self.client, value)
    }

    /// Like [`Handle::call`], but the result goes through the etag cache.
    pub async fn call_cached<T: FromWire>(
        &self,
        name: &str,
        args: impl Into<Option<Value>>,
    ) -> Result<T> {
        let value = self
            .client
            .send_cached_command(self.command(name, args.into()))
            .await?;
        T::from_wire(&self.client, value)
    }

    fn command(&self, name: &str, args: Option<Value>) -> Command {
        let mut command = Command::new(self.ns, name).nsid(self.nsid());
        command.args = args;
        command
    }
}

/// Live's convention for "append": index `-1`.
pub(crate) fn index_or_end(index: Option<usize>) -> i64 {
    index.map_or(-1, |index| index as i64)
}

/// Implemented by every Live object.
pub trait LiveObject {
    /// Low-level access to the object.
    fn handle(&self) -> &Handle;

    /// Id of the Live object; empty for namespaces without one.
    fn live_id(&self) -> &str {
        self.handle().nsid().unwrap_or_default()
    }
}

/// Something that identifies a Live object: the object itself or its id.
///
/// Methods that take another object (`track.delete_clip(&clip)`) accept
/// either, like the JS API.
pub trait IntoLiveId {
    /// The id of the referenced object.
    fn into_live_id(self) -> String;
}

impl IntoLiveId for &str {
    fn into_live_id(self) -> String {
        self.to_owned()
    }
}

impl IntoLiveId for String {
    fn into_live_id(self) -> String {
        self
    }
}

impl IntoLiveId for &String {
    fn into_live_id(self) -> String {
        self.clone()
    }
}

impl<T: LiveObject> IntoLiveId for &T {
    fn into_live_id(self) -> String {
        self.live_id().to_owned()
    }
}
