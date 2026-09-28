//! Client configuration.

use std::time::Duration;

/// Default WebSocket host of the Remote Script.
pub const DEFAULT_HOST: &str = "127.0.0.1";
/// Default WebSocket port of the Remote Script.
pub const DEFAULT_PORT: u16 = 39031;

/// Options for [`crate::Ableton::new`].
///
/// Build it with struct update syntax:
///
/// ```
/// # use ableton_core::AbletonOptions;
/// # use std::time::Duration;
/// let options = AbletonOptions {
///     password: Some("secret".into()),
///     command_timeout: Duration::from_secs(5),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct AbletonOptions {
    /// WebSocket host of the Remote Script.
    pub host: String,

    /// WebSocket port of the Remote Script.
    pub port: u16,

    /// Shared secret matching `PASSWORD` in `Config.py`. Sent as an
    /// HMAC-SHA256 of the per-connection salt the plugin hands out.
    pub password: Option<String>,

    /// How often to ping the Remote Script to check that it's still
    /// reachable. Pings are skipped while other messages keep arriving.
    pub heartbeat_interval: Duration,

    /// How long to wait for the WebSocket handshake before aborting and
    /// retrying.
    pub connect_timeout: Duration,

    /// How long to wait for the reply to a command before failing with
    /// [`crate::Error::Timeout`].
    pub command_timeout: Duration,

    /// Log a warning when a reply takes longer than this.
    pub command_warn: Duration,

    /// Log a warning when a message took longer than this to travel from
    /// the Python script to the client.
    pub message_latency_warn: Duration,

    /// Response cache used by cached property reads. `None` disables it.
    pub cache: Option<CacheOptions>,
}

impl Default for AbletonOptions {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            password: None,
            heartbeat_interval: Duration::from_millis(2000),
            connect_timeout: Duration::from_millis(5000),
            command_timeout: Duration::from_millis(3000),
            command_warn: Duration::from_millis(2000),
            message_latency_warn: Duration::from_millis(500),
            cache: Some(CacheOptions::default()),
        }
    }
}

impl AbletonOptions {
    pub(crate) fn url(&self) -> String {
        format!("ws://{}:{}", self.host, self.port)
    }
}

/// Settings of the etag-based response cache.
#[derive(Debug, Clone, Copy)]
pub struct CacheOptions {
    /// Maximum number of cached responses.
    pub max_entries: usize,
    /// How long a cached response stays valid.
    pub ttl: Duration,
}

impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            max_entries: 500,
            ttl: Duration::from_secs(10 * 60),
        }
    }
}
