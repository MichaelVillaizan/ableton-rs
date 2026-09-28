//! The client: connection lifecycle and the low-level command API.
//!
//! Internally the client is a small set of cooperating tasks that share one
//! `Inner` state:
//!
//! - the **connection task** (`connection.rs`) owns the WebSocket, reconnects
//!   with backoff and feeds incoming frames to `Inner::handle_incoming`;
//! - the **batcher** (`commands.rs`) coalesces commands issued at the same
//!   time into one envelope and matches replies to callers;
//! - the **heartbeat** pings Live while it's otherwise quiet.
//!
//! Tasks only hold weak references, so dropping the last [`Ableton`] handle
//! shuts everything down. Calling [`Ableton::close`] does the same, but
//! gracefully.

mod commands;
mod connection;
mod listeners;

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use self::commands::{PendingReply, QueuedCommand};
use self::connection::Control;
use self::listeners::{ListenerRegistry, Removal};
use crate::PLUGIN_VERSION;
use crate::auth::hmac_sha256_hex;
use crate::cache::{CachedResponse, ResponseCache};
use crate::error::{Error, Result};
use crate::events::{ClientEvent, ConnectKind, DisconnectKind};
use crate::ns::{Application, Internal, Midi, Session, Song};
use crate::options::AbletonOptions;
use crate::protocol::{Command, ConnectData};

/// Capacity of the lifecycle event channel. Slow receivers skip events.
const EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Client for the AbletonJS Remote Script.
///
/// Cheap to clone: every clone talks through the same connection. Connect with
/// [`Ableton::start`], then use the namespaces ([`Ableton::song`], …).
#[derive(Clone)]
pub struct Ableton {
    inner: Arc<Inner>,
}

/// A handle that doesn't keep the client alive.
#[derive(Clone)]
pub struct WeakAbleton {
    inner: Weak<Inner>,
}

impl WeakAbleton {
    /// Returns the client if it's still alive.
    pub fn upgrade(&self) -> Option<Ableton> {
        self.inner.upgrade().map(|inner| Ableton { inner })
    }
}

impl fmt::Debug for Ableton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ableton")
            .field("url", &self.inner.options.url())
            .field("connected", &self.is_connected())
            .finish_non_exhaustive()
    }
}

impl Default for Ableton {
    fn default() -> Self {
        Self::new(AbletonOptions::default())
    }
}

impl Ableton {
    /// Creates a client. Nothing happens until [`Ableton::start`] is called.
    pub fn new(options: AbletonOptions) -> Self {
        Self {
            inner: Arc::new(Inner::new(options)),
        }
    }

    /// The options this client was created with.
    pub fn options(&self) -> &AbletonOptions {
        &self.inner.options
    }

    /// The current Live Set (tracks, scenes, tempo, playback, …).
    pub fn song(&self) -> Song {
        Song::new(self)
    }

    /// Red box / session ring control.
    pub fn session(&self) -> Session {
        Session::new(self)
    }

    /// Live application metadata, dialogs, browser and view.
    pub fn application(&self) -> Application {
        Application::new(self)
    }

    /// Internal plugin helpers (ping, version).
    pub fn internal(&self) -> Internal {
        Internal::new(self)
    }

    /// Forwarded MIDI note/CC tracking.
    pub fn midi(&self) -> Midi {
        Midi::new(self)
    }

    /// Starts the client and waits for a connection with Live.
    ///
    /// With a `timeout`, gives up (and closes the client) when no connection
    /// could be established in time. Keep it above two seconds to avoid
    /// false positives. Calling `start` on a running client just waits for
    /// the connection.
    pub async fn start(&self, timeout: Option<Duration>) -> Result<()> {
        let inner = &self.inner;
        let already_running = {
            let mut state = inner.state();
            let running = state.lifecycle != Lifecycle::Closed;
            if running {
                warn!(
                    state = ?state.lifecycle,
                    "Tried calling start, but the client is already running"
                );
            } else {
                state.lifecycle = Lifecycle::Starting;
                inner.spawn_tasks(&mut state);
            }
            running
        };
        if already_running {
            return self.wait_for_connection().await;
        }
        info!(url = %inner.options.url(), "Connecting to Live");
        let connected = match timeout {
            Some(timeout) => tokio::time::timeout(timeout, self.wait_for_connection())
                .await
                .unwrap_or(Err(Error::ConnectionTimeout)),
            None => self.wait_for_connection().await,
        };
        if let Err(error) = connected {
            self.close().await;
            return Err(error);
        }

        info!("Got connection!");
        {
            let mut state = inner.state();
            state.lifecycle = Lifecycle::Started;
            state.background_tasks.push(tokio::spawn(run_heartbeat(
                Arc::downgrade(inner),
                inner.options.heartbeat_interval,
            )));
        }
        inner.handle_connect(ConnectKind::Start);

        let client = self.downgrade();
        tokio::spawn(async move {
            let Some(client) = client.upgrade() else {
                return;
            };
            if let Ok(version) = client.internal().version().await {
                if version != PLUGIN_VERSION {
                    warn!(
                        "The installed version of your AbletonJS plugin ({version}) is different \
                         from the one this client expects ({PLUGIN_VERSION}). Please update your \
                         AbletonJS plugin to the latest version: https://git.io/JvaOu"
                    );
                }
            }
        });

        Ok(())
    }

    /// Closes the client, flushing messages that are already queued.
    pub async fn close(&self) {
        info!("Closing the client");
        let (control, connection_task, background_tasks) = {
            let mut state = self.inner.state();
            state.should_reconnect = false;
            state.commands = None;
            (
                state.control.take(),
                state.connection_task.take(),
                std::mem::take(&mut state.background_tasks),
            )
        };

        for task in background_tasks {
            task.abort();
        }
        if let Some(control) = control {
            let _ = control.send(Control::Shutdown);
        }
        if let Some(task) = connection_task {
            let _ = task.await;
        }

        {
            let mut state = self.inner.state();
            state.lifecycle = Lifecycle::Closed;
            state.connected = false;
        }
        info!("Client closed");

        // Unblock start()/wait_for_connection() calls still waiting for this
        // connection attempt, since we just gave up on it.
        self.inner.settle_waiters(Err(Error::Closed));
    }

    /// Returns immediately when connected, otherwise waits for the next
    /// connection.
    pub async fn wait_for_connection(&self) -> Result<()> {
        self.inner.wait_for_connection().await
    }

    /// Whether the client currently has an active connection to Live.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Round-trip time of the last heartbeat. A rough measurement.
    pub fn get_ping(&self) -> Duration {
        Duration::from_micros(self.inner.latency_us.load(Ordering::Relaxed))
    }

    /// Subscribes to lifecycle events (connect, disconnect, ping, …).
    pub fn events(&self) -> broadcast::Receiver<ClientEvent> {
        self.inner.events.subscribe()
    }

    /// Sends a raw command to Live. Usually you won't need this: the typed
    /// namespaces are a better starting point.
    ///
    /// Commands issued at the same time are coalesced into a single
    /// WebSocket round-trip.
    pub async fn send_command(&self, command: Command) -> Result<Value> {
        self.inner.send_command(command).await
    }

    /// Sends a command through the etag response cache.
    pub async fn send_cached_command(&self, command: Command) -> Result<Value> {
        self.inner.send_cached_command(command).await
    }

    /// Reads a property of a Live object. With `cache`, uses the response
    /// cache (when it's enabled).
    pub async fn get_prop(
        &self,
        ns: &str,
        nsid: Option<&str>,
        prop: &str,
        cache: bool,
    ) -> Result<Value> {
        let command = Command::new(ns, "get_prop")
            .nsid(nsid)
            .args(json!({ "prop": prop }));

        if cache && self.inner.cache.is_some() {
            self.send_cached_command(command).await
        } else {
            self.send_command(command).await
        }
    }

    /// Sets a property of a Live object.
    pub async fn set_prop(
        &self,
        ns: &str,
        nsid: Option<&str>,
        prop: &str,
        value: Value,
    ) -> Result<Value> {
        let command = Command::new(ns, "set_prop")
            .nsid(nsid)
            .args(json!({ "prop": prop, "value": value }));
        self.send_command(command).await
    }

    /// Calls `listener` whenever a property of a Live object changes.
    ///
    /// Listeners run on the connection task, so keep them short and hand
    /// heavy work off to another task. They are dropped when Live
    /// disconnects.
    pub async fn add_prop_listener<F>(
        &self,
        ns: &str,
        nsid: Option<&str>,
        prop: &str,
        listener: F,
    ) -> Result<Subscription>
    where
        F: Fn(Value) + Send + Sync + 'static,
    {
        let requested_id = self.inner.next_id();
        let mut args = listener_args(prop, nsid);
        args.insert("eventId".into(), requested_id.into());

        let command = Command::new(ns, "add_listener")
            .nsid(nsid)
            .args(Value::Object(args));
        // The script hands back the existing event id when this connection
        // already observes the property.
        let event_id = match self.send_command(command).await? {
            Value::String(id) => id,
            other => other.to_string(),
        };

        let listener_id = self.inner.listeners.add(&event_id, Arc::new(listener));
        Ok(Subscription {
            client: self.clone(),
            ns: ns.to_owned(),
            nsid: nsid.map(str::to_owned),
            prop: prop.to_owned(),
            event_id,
            listener_id,
        })
    }

    /// Removes a property listener. Returns `false` when it was already gone.
    ///
    /// Usually you'd call [`Subscription::unsubscribe`] instead.
    pub async fn remove_prop_listener(&self, subscription: &Subscription) -> Result<bool> {
        match self
            .inner
            .listeners
            .remove(&subscription.event_id, subscription.listener_id)
        {
            Removal::NotFound => Ok(false),
            Removal::Removed => Ok(true),
            Removal::RemovedLast => {
                let nsid = subscription.nsid.as_deref();
                let command = Command::new(&subscription.ns, "remove_listener")
                    .nsid(nsid)
                    .args(Value::Object(listener_args(&subscription.prop, nsid)));
                self.send_command(command).await?;
                Ok(true)
            }
        }
    }

    /// Forgets every property listener locally, e.g. after a disconnect.
    pub fn remove_all_prop_listeners(&self) {
        self.inner.listeners.clear();
    }

    /// Sends a raw text frame, bypassing batching. Use with caution.
    pub fn send_raw(&self, message: impl Into<String>) -> Result<()> {
        self.inner.send_raw(message.into())
    }

    /// Drops every cached response.
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.inner.cache {
            cache.clear();
        }
    }

    /// A handle that doesn't keep the client alive.
    pub fn downgrade(&self) -> WeakAbleton {
        WeakAbleton {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

fn listener_args(prop: &str, nsid: Option<&str>) -> Map<String, Value> {
    let mut args = Map::new();
    args.insert("prop".into(), prop.into());
    if let Some(nsid) = nsid {
        args.insert("nsid".into(), nsid.into());
    }
    args
}

/// An active property listener, returned by `on_*` methods and
/// [`Ableton::add_prop_listener`].
///
/// Dropping it doesn't remove the listener; call
/// [`Subscription::unsubscribe`] for that.
#[derive(Debug)]
#[must_use = "the listener stays active until `unsubscribe` is called"]
pub struct Subscription {
    client: Ableton,
    ns: String,
    nsid: Option<String>,
    prop: String,
    event_id: String,
    listener_id: u64,
}

impl Subscription {
    /// The observed property.
    pub fn prop(&self) -> &str {
        &self.prop
    }

    /// Id of the listener events sent by the Remote Script.
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    /// Removes the listener. Returns `false` when it was already gone, e.g.
    /// because Live disconnected in the meantime.
    pub async fn unsubscribe(self) -> Result<bool> {
        self.client.remove_prop_listener(&self).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lifecycle {
    Closed,
    Starting,
    Started,
}

struct State {
    lifecycle: Lifecycle,
    connected: bool,
    should_reconnect: bool,
    /// Outgoing frames of the open socket.
    socket: Option<mpsc::UnboundedSender<Message>>,
    /// Instructions for the connection task.
    control: Option<mpsc::UnboundedSender<Control>>,
    /// Queue of the batcher.
    commands: Option<mpsc::UnboundedSender<QueuedCommand>>,
    connection_task: Option<JoinHandle<()>>,
    background_tasks: Vec<JoinHandle<()>>,
    /// Pending `wait_for_connection` calls.
    waiters: Vec<oneshot::Sender<Result<()>>>,
}

pub(crate) struct Inner {
    options: AbletonOptions,
    state: Mutex<State>,
    /// Envelopes waiting for their reply, by uuid.
    pending: Mutex<HashMap<String, PendingReply>>,
    listeners: ListenerRegistry,
    cache: Option<ResponseCache>,
    events: broadcast::Sender<ClientEvent>,
    next_id: AtomicU64,
    created_at: Instant,
    /// Milliseconds since `created_at` + 1 of the last received message;
    /// 0 when nothing was received yet.
    last_message_at: AtomicU64,
    latency_us: AtomicU64,
    /// Bumped whenever Live greets us; heartbeats started before that don't
    /// count as failures anymore.
    heartbeat_epoch: AtomicU64,
}

impl Inner {
    fn new(options: AbletonOptions) -> Self {
        let (events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            cache: options.cache.map(ResponseCache::new),
            options,
            state: Mutex::new(State {
                lifecycle: Lifecycle::Closed,
                connected: false,
                should_reconnect: false,
                socket: None,
                control: None,
                commands: None,
                connection_task: None,
                background_tasks: Vec::new(),
                waiters: Vec::new(),
            }),
            pending: Mutex::default(),
            listeners: ListenerRegistry::default(),
            events,
            next_id: AtomicU64::new(1),
            created_at: Instant::now(),
            last_message_at: AtomicU64::new(0),
            latency_us: AtomicU64::new(0),
            heartbeat_epoch: AtomicU64::new(0),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect("client state lock poisoned")
    }

    /// Starts the batcher and the connection task.
    fn spawn_tasks(self: &Arc<Self>, state: &mut State) {
        state.should_reconnect = true;

        let (commands, queue) = mpsc::unbounded_channel();
        state.commands = Some(commands);
        state
            .background_tasks
            .push(tokio::spawn(commands::run_batcher(
                Arc::downgrade(self),
                queue,
            )));

        let (control, control_rx) = mpsc::unbounded_channel();
        state.control = Some(control);
        state.connection_task = Some(tokio::spawn(connection::run(
            Arc::downgrade(self),
            control_rx,
        )));
    }

    fn pending(&self) -> MutexGuard<'_, HashMap<String, PendingReply>> {
        self.pending.lock().expect("pending lock poisoned")
    }

    fn next_id(&self) -> String {
        self.next_id.fetch_add(1, Ordering::Relaxed).to_string()
    }

    fn emit(&self, event: ClientEvent) {
        // Nobody listening is fine.
        let _ = self.events.send(event);
    }

    fn is_connected(&self) -> bool {
        self.state().connected
    }

    fn should_reconnect(&self) -> bool {
        self.state().should_reconnect
    }

    fn touch(&self) {
        let elapsed = self.created_at.elapsed().as_millis() as u64;
        self.last_message_at.store(elapsed + 1, Ordering::Relaxed);
    }

    fn since_last_message(&self) -> Duration {
        match self.last_message_at.load(Ordering::Relaxed) {
            0 => Duration::MAX,
            at => self
                .created_at
                .elapsed()
                .saturating_sub(Duration::from_millis(at - 1)),
        }
    }

    // --- connection state -------------------------------------------------

    fn attach_socket(&self, socket: mpsc::UnboundedSender<Message>) {
        self.state().socket = Some(socket);
    }

    fn detach_socket(&self) {
        self.state().socket = None;
    }

    /// Asks the connection task to drop the current socket and reconnect.
    fn request_socket_drop(&self, kind: DisconnectKind) {
        if let Some(control) = &self.state().control {
            let _ = control.send(Control::Drop(kind));
        }
    }

    fn handle_connect(&self, kind: ConnectKind) {
        let waiters = {
            let mut state = self.state();
            if state.connected {
                return;
            }
            state.connected = true;
            std::mem::take(&mut state.waiters)
        };

        info!(?kind, "Live connected");
        self.emit(ClientEvent::Connect(kind));
        for waiter in waiters {
            let _ = waiter.send(Ok(()));
        }
    }

    fn handle_disconnect(&self, kind: DisconnectKind) {
        {
            let mut state = self.state();
            if !state.connected {
                return;
            }
            state.connected = false;
        }

        self.listeners.clear();
        if let Some(cache) = &self.cache {
            cache.clear();
        }

        // After missed heartbeats, keep pending commands: Live might just be
        // hanging for a moment and still answer them.
        if kind == DisconnectKind::Realtime {
            let pending: Vec<_> = self.pending().drain().map(|(_, reply)| reply).collect();
            for reply in pending {
                reply.disconnect();
            }
        }

        info!(?kind, "Live disconnected");
        self.emit(ClientEvent::Disconnect(kind));
    }

    fn settle_waiters(&self, result: Result<()>) {
        let waiters = std::mem::take(&mut self.state().waiters);
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
    }

    async fn wait_for_connection(&self) -> Result<()> {
        let waiter = {
            let mut state = self.state();
            if state.connected {
                return Ok(());
            }
            let (tx, rx) = oneshot::channel();
            state.waiters.push(tx);
            rx
        };
        waiter.await.unwrap_or(Err(Error::Closed))
    }

    // --- sending ------------------------------------------------------------

    fn send_raw(&self, message: String) -> Result<()> {
        let state = self.state();
        if state.lifecycle == Lifecycle::Closed {
            return Err(Error::NotStarted);
        }
        let socket = state.socket.as_ref().ok_or(Error::NotConnected)?;
        socket
            .send(Message::Text(message.into()))
            .map_err(|_| Error::NotConnected)
    }

    async fn send_command(&self, command: Command) -> Result<Value> {
        let (reply, response) = oneshot::channel();
        let queue = self.state().commands.clone().ok_or(Error::NotStarted)?;
        queue
            .send(QueuedCommand { command, reply })
            .map_err(|_| Error::NotStarted)?;
        response.await.unwrap_or(Err(Error::Closed))
    }

    async fn send_cached_command(&self, mut command: Command) -> Result<Value> {
        let key = ResponseCache::key(&command);
        let cached = self.cache.as_ref().and_then(|cache| cache.get(&key));

        command.etag = cached.as_ref().map(|c| c.etag.clone());
        command.cache = Some(true);

        let Value::Object(mut result) = self.send_command(command).await? else {
            return Err(Error::Protocol(
                "cached command didn't return an object".into(),
            ));
        };

        if result.contains_key("__cached") {
            return cached.map(|c| c.data).ok_or_else(|| {
                Error::Protocol("Tried to get an object that isn't cached.".into())
            });
        }

        let data = result.remove("data").unwrap_or(Value::Null);
        if let (Some(cache), Some(Value::String(etag))) = (&self.cache, result.remove("etag")) {
            if !etag.is_empty() {
                cache.set(
                    key,
                    CachedResponse {
                        etag,
                        data: data.clone(),
                    },
                );
            }
        }
        Ok(data)
    }

    // --- Live greeting & authentication ---------------------------------------

    async fn handle_server_connect(self: Arc<Self>, data: Value) {
        self.heartbeat_epoch.fetch_add(1, Ordering::SeqCst);

        let data: ConnectData = serde_json::from_value(data).unwrap_or_default();
        if let Some(port) = data.port.filter(|port| *port != self.options.port) {
            info!(port, "Got server port via connect");
        }

        if data.requires_auth {
            if let Err(error) = self.authenticate(data.salt.as_deref()).await {
                self.abort_authentication(error);
                return;
            }
        }

        let kind = if self.state().lifecycle == Lifecycle::Starting {
            ConnectKind::Start
        } else {
            ConnectKind::Realtime
        };
        self.handle_connect(kind);
    }

    async fn authenticate(self: &Arc<Self>, salt: Option<&str>) -> Result<()> {
        let Some(password) = &self.options.password else {
            return Err(Error::Auth(
                "The AbletonJS plugin requires a password. Pass it in the options.".into(),
            ));
        };
        let Some(salt) = salt else {
            return Err(Error::Auth(
                "The AbletonJS plugin did not send an authentication salt.".into(),
            ));
        };

        let command = Command::new("internal", "authenticate")
            .args(json!({ "hash": hmac_sha256_hex(password, salt) }));

        // Sent on its own instead of through the batcher: the script only
        // accepts envelopes that start with the authentication command.
        let results = self
            .send_envelope(vec![command])
            .await
            .and_then(|value| commands::split_results(value, 1))
            .map_err(|error| Error::Auth(error.to_string()))?;

        match results.into_iter().next() {
            Some(Ok(_)) => Ok(()),
            Some(Err(error)) => Err(Error::Auth(error.to_string())),
            None => Err(Error::Auth("empty reply".into())),
        }
    }

    fn abort_authentication(&self, error: Error) {
        error!("{error}");
        {
            let mut state = self.state();
            state.should_reconnect = false;
            state.lifecycle = Lifecycle::Closed;
        }
        self.emit(ClientEvent::Error(error.clone()));
        self.settle_waiters(Err(error));
        self.request_socket_drop(DisconnectKind::Realtime);
    }

    // --- heartbeat ------------------------------------------------------------

    async fn heartbeat(self: &Arc<Self>) {
        let has_socket = {
            let state = self.state();
            state.connected && state.socket.is_some()
        };
        // Skip the ping when we heard from Live recently.
        if !has_socket || self.since_last_message() < self.options.heartbeat_interval {
            return;
        }

        let epoch = self.heartbeat_epoch.load(Ordering::SeqCst);
        let started = Instant::now();
        let ping = Command::new("internal", "get_prop").args(json!({ "prop": "ping" }));

        match self.send_command(ping).await {
            Ok(_) => {
                self.handle_connect(ConnectKind::Heartbeat);
                let latency = started.elapsed();
                self.latency_us
                    .store(latency.as_micros() as u64, Ordering::Relaxed);
                self.emit(ClientEvent::Ping(latency));
            }
            Err(error) => {
                if epoch != self.heartbeat_epoch.load(Ordering::SeqCst) {
                    debug!("Cancelled heartbeat");
                } else if self.is_connected() {
                    warn!(%error, "Heartbeat failed");
                    self.request_socket_drop(DisconnectKind::Heartbeat);
                }
            }
        }
    }
}

async fn run_heartbeat(inner: Weak<Inner>, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let Some(inner) = inner.upgrade() else {
            return;
        };
        inner.heartbeat().await;
    }
}
