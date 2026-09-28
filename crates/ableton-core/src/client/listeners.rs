//! Callbacks registered for property listener events.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

pub(crate) type Callback = Arc<dyn Fn(Value) + Send + Sync>;

/// What happened when a callback was removed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Removal {
    /// No such callback.
    NotFound,
    /// Removed; other callbacks still use the same event id.
    Removed,
    /// Removed the last callback of the event id; the Remote Script should
    /// stop sending it.
    RemovedLast,
}

/// Maps listener event ids (as returned by `add_listener`) to callbacks.
///
/// Several callbacks can share one event id: the Remote Script only keeps one
/// listener per connection and property, and hands back the existing event
/// id when the same property is observed again.
#[derive(Default)]
pub(crate) struct ListenerRegistry {
    next_id: AtomicU64,
    callbacks: Mutex<HashMap<String, Vec<(u64, Callback)>>>,
}

impl ListenerRegistry {
    pub fn add(&self, event_id: &str, callback: Callback) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.lock()
            .entry(event_id.to_owned())
            .or_default()
            .push((id, callback));
        id
    }

    pub fn remove(&self, event_id: &str, listener_id: u64) -> Removal {
        let mut callbacks = self.lock();
        let Some(list) = callbacks.get_mut(event_id) else {
            return Removal::NotFound;
        };
        let Some(index) = list.iter().position(|(id, _)| *id == listener_id) else {
            return Removal::NotFound;
        };

        list.remove(index);
        if list.is_empty() {
            callbacks.remove(event_id);
            Removal::RemovedLast
        } else {
            Removal::Removed
        }
    }

    /// Calls every callback registered for `event_id`. Returns `false` when
    /// there were none.
    pub fn dispatch(&self, event_id: &str, data: &Value) -> bool {
        // Clone the callbacks out so they can add or remove listeners
        // themselves without deadlocking.
        let callbacks: Vec<Callback> = match self.lock().get(event_id) {
            Some(list) => list.iter().map(|(_, cb)| Arc::clone(cb)).collect(),
            None => return false,
        };
        for callback in callbacks {
            callback(data.clone());
        }
        true
    }

    pub fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<(u64, Callback)>>> {
        self.callbacks.lock().expect("listener lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use serde_json::json;

    use super::*;

    #[test]
    fn keeps_the_event_until_the_last_callback_is_removed() {
        let registry = ListenerRegistry::default();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = |hits: &Arc<AtomicUsize>| -> Callback {
            let hits = Arc::clone(hits);
            Arc::new(move |_| {
                hits.fetch_add(1, Ordering::SeqCst);
            })
        };

        let a = registry.add("7", counter(&hits));
        let b = registry.add("7", counter(&hits));
        assert!(registry.dispatch("7", &json!(1)));
        assert_eq!(hits.load(Ordering::SeqCst), 2);

        assert_eq!(registry.remove("7", a), Removal::Removed);
        assert_eq!(registry.remove("7", a), Removal::NotFound);
        assert_eq!(registry.remove("7", b), Removal::RemovedLast);
        assert!(!registry.dispatch("7", &json!(1)));
    }
}
