//! Commands invoked by the webview as `plugin:ableton|<name>`.
//!
//! They mirror the protocol-level API of the JS client (`getProp`,
//! `setProp`, `sendCommand`, `addPropListener`), which is enough to drive
//! every namespace from the frontend.

use std::time::Duration;

use ableton_core::protocol::Command;
use serde::Serialize;
use serde_json::Value;
use tauri::State;
use tauri::ipc::Channel;

use crate::error::{Error, Result};
use crate::state::AbletonState;

/// Starts the client and waits for Live.
#[tauri::command]
pub(crate) async fn start(state: State<'_, AbletonState>, timeout_ms: Option<u64>) -> Result<()> {
    state
        .client()
        .start(timeout_ms.map(Duration::from_millis))
        .await?;
    Ok(())
}

/// Closes the client.
#[tauri::command]
pub(crate) async fn close(state: State<'_, AbletonState>) -> Result<()> {
    state.clear_subscriptions();
    state.client().close().await;
    Ok(())
}

#[tauri::command]
pub(crate) fn is_connected(state: State<'_, AbletonState>) -> bool {
    state.client().is_connected()
}

/// Heartbeat round-trip time in milliseconds.
#[tauri::command]
pub(crate) fn get_ping(state: State<'_, AbletonState>) -> f64 {
    state.client().get_ping().as_secs_f64() * 1000.0
}

#[tauri::command]
pub(crate) async fn get_prop(
    state: State<'_, AbletonState>,
    ns: String,
    nsid: Option<String>,
    prop: String,
    cache: Option<bool>,
) -> Result<Value> {
    Ok(state
        .client()
        .get_prop(&ns, nsid.as_deref(), &prop, cache.unwrap_or(false))
        .await?)
}

#[tauri::command]
pub(crate) async fn set_prop(
    state: State<'_, AbletonState>,
    ns: String,
    nsid: Option<String>,
    prop: String,
    value: Value,
) -> Result<Value> {
    Ok(state
        .client()
        .set_prop(&ns, nsid.as_deref(), &prop, value)
        .await?)
}

/// Calls any function of a Live object.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_command(
    state: State<'_, AbletonState>,
    ns: String,
    nsid: Option<String>,
    name: String,
    args: Option<Value>,
    timeout_ms: Option<u64>,
    cache: Option<bool>,
) -> Result<Value> {
    let mut command = Command::new(ns, name).nsid(nsid);
    command.args = args;
    if let Some(timeout) = timeout_ms {
        command = command.timeout(Duration::from_millis(timeout));
    }

    let client = state.client();
    let result = if cache.unwrap_or(false) {
        client.send_cached_command(command).await
    } else {
        client.send_command(command).await
    };
    Ok(result?)
}

/// A listener registered by the webview.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Listener {
    /// Pass it to `remove_listener`.
    id: u64,
    /// Event id used by the Remote Script.
    event_id: String,
}

/// Streams every change of a property through `on_event`.
#[tauri::command]
pub(crate) async fn add_listener(
    state: State<'_, AbletonState>,
    ns: String,
    nsid: Option<String>,
    prop: String,
    on_event: Channel<Value>,
) -> Result<Listener> {
    let subscription = state
        .client()
        .add_prop_listener(&ns, nsid.as_deref(), &prop, move |value| {
            // The webview may be gone already; nothing to do then.
            let _ = on_event.send(value);
        })
        .await?;

    let event_id = subscription.event_id().to_owned();
    let id = state.add_subscription(subscription);
    Ok(Listener { id, event_id })
}

/// Removes a listener. Returns `false` when Live already dropped it.
#[tauri::command]
pub(crate) async fn remove_listener(state: State<'_, AbletonState>, id: u64) -> Result<bool> {
    let subscription = state
        .take_subscription(id)
        .ok_or(Error::UnknownListener(id))?;
    Ok(subscription.unsubscribe().await?)
}
