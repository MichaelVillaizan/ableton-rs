//! Etag-based response cache.
//!
//! Cached reads send the etag of the last response they saw. When the value
//! didn't change, the script answers `{ "__cached": true }` instead of
//! resending it, and the client serves the value from here.

use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;
use serde_json::Value;

use crate::options::CacheOptions;
use crate::protocol::Command;

#[derive(Debug, Clone)]
pub(crate) struct CachedResponse {
    pub etag: String,
    pub data: Value,
}

struct Entry {
    response: CachedResponse,
    stored_at: Instant,
}

pub(crate) struct ResponseCache {
    entries: Mutex<LruCache<String, Entry>>,
    ttl: Duration,
}

impl ResponseCache {
    pub fn new(options: CacheOptions) -> Self {
        let capacity = NonZeroUsize::new(options.max_entries).unwrap_or(NonZeroUsize::MIN);
        Self {
            entries: Mutex::new(LruCache::new(capacity)),
            ttl: options.ttl,
        }
    }

    /// Cache key of a command: `ns/nsid/prop`, or the JSON arguments when the
    /// command doesn't read a property.
    pub fn key(command: &Command) -> String {
        let args = command.args.as_ref().map(|args| match args.get("prop") {
            Some(Value::String(prop)) => prop.clone(),
            Some(prop) => prop.to_string(),
            None => args.to_string(),
        });

        [
            Some(command.ns.as_str()),
            command.nsid.as_deref(),
            args.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
    }

    pub fn get(&self, key: &str) -> Option<CachedResponse> {
        let mut entries = self.entries.lock().expect("cache lock poisoned");
        let expired = entries.get(key)?.stored_at.elapsed() > self.ttl;
        if expired {
            entries.pop(key);
            return None;
        }
        entries.get(key).map(|entry| entry.response.clone())
    }

    pub fn set(&self, key: String, response: CachedResponse) {
        let entry = Entry {
            response,
            stored_at: Instant::now(),
        };
        self.entries
            .lock()
            .expect("cache lock poisoned")
            .put(key, entry);
    }

    pub fn clear(&self) {
        self.entries.lock().expect("cache lock poisoned").clear();
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keys_property_reads_by_prop_name() {
        let command = Command::new("track", "get_prop")
            .nsid(Some("live_1"))
            .args(json!({ "prop": "devices" }));
        assert_eq!(ResponseCache::key(&command), "track/live_1/devices");
    }

    #[test]
    fn keys_other_commands_by_their_arguments() {
        let command = Command::new("song", "get_data").args(json!({ "key": "a" }));
        assert_eq!(ResponseCache::key(&command), r#"song/{"key":"a"}"#);
    }

    #[test]
    fn drops_expired_entries() {
        let cache = ResponseCache::new(CacheOptions {
            max_entries: 10,
            ttl: Duration::ZERO,
        });
        cache.set(
            "k".into(),
            CachedResponse {
                etag: "e".into(),
                data: json!(1),
            },
        );
        std::thread::sleep(Duration::from_millis(2));
        assert!(cache.get("k").is_none());
    }
}
