use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use ableton_core::Ableton;
use ableton_core::client::Subscription;

/// State managed by the plugin: the client plus the listeners the webview
/// registered, by id.
pub(crate) struct AbletonState {
    client: Ableton,
    subscriptions: Mutex<HashMap<u64, Subscription>>,
    next_id: AtomicU64,
}

impl AbletonState {
    pub fn new(client: Ableton) -> Self {
        Self {
            client,
            subscriptions: Mutex::default(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn client(&self) -> &Ableton {
        &self.client
    }

    pub fn add_subscription(&self, subscription: Subscription) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.lock().insert(id, subscription);
        id
    }

    pub fn take_subscription(&self, id: u64) -> Option<Subscription> {
        self.lock().remove(&id)
    }

    pub fn clear_subscriptions(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Subscription>> {
        self.subscriptions
            .lock()
            .expect("subscription lock poisoned")
    }
}
