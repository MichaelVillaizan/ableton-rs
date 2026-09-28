use std::time::Duration;

use ableton_core::{AbletonOptions, CacheOptions};
use serde::Deserialize;

/// Plugin configuration, read from `plugins.ableton` in `tauri.conf.json`.
/// Every field is optional and falls back to the `ableton-core` default.
///
/// ```json
/// {
///   "plugins": {
///     "ableton": { "port": 39031, "password": "secret", "commandTimeoutMs": 5000 }
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub password: Option<String>,
    pub heartbeat_interval_ms: Option<u64>,
    pub connect_timeout_ms: Option<u64>,
    pub command_timeout_ms: Option<u64>,
    pub command_warn_ms: Option<u64>,
    pub message_latency_warn_ms: Option<u64>,
    pub disable_cache: bool,
    pub cache_max_entries: Option<usize>,
    pub cache_ttl_ms: Option<u64>,
    /// Connect as soon as the app starts (default: true).
    pub auto_start: Option<bool>,
}

impl Config {
    /// Client options described by this configuration.
    pub fn to_options(&self) -> AbletonOptions {
        let defaults = AbletonOptions::default();
        let millis =
            |value: Option<u64>, default: Duration| value.map_or(default, Duration::from_millis);

        let cache = (!self.disable_cache).then(|| {
            let defaults = CacheOptions::default();
            CacheOptions {
                max_entries: self.cache_max_entries.unwrap_or(defaults.max_entries),
                ttl: millis(self.cache_ttl_ms, defaults.ttl),
            }
        });

        AbletonOptions {
            host: self.host.clone().unwrap_or(defaults.host),
            port: self.port.unwrap_or(defaults.port),
            password: self.password.clone(),
            heartbeat_interval: millis(self.heartbeat_interval_ms, defaults.heartbeat_interval),
            connect_timeout: millis(self.connect_timeout_ms, defaults.connect_timeout),
            command_timeout: millis(self.command_timeout_ms, defaults.command_timeout),
            command_warn: millis(self.command_warn_ms, defaults.command_warn),
            message_latency_warn: millis(
                self.message_latency_warn_ms,
                defaults.message_latency_warn,
            ),
            cache,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_camel_case_and_keeps_defaults() {
        let config: Config = serde_json::from_value(serde_json::json!({
            "port": 1234,
            "commandTimeoutMs": 5000,
            "disableCache": true
        }))
        .unwrap();
        let options = config.to_options();

        assert_eq!(options.port, 1234);
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.command_timeout, Duration::from_secs(5));
        assert_eq!(
            options.connect_timeout,
            AbletonOptions::default().connect_timeout
        );
        assert!(options.cache.is_none());
    }
}
