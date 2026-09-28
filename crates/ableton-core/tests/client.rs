mod support;

use std::time::Duration;

use ableton_core::prelude::*;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use support::{FakeLive, default_handler};

async fn connect(options: AbletonOptions) -> Ableton {
    let ableton = Ableton::new(options);
    ableton
        .start(Some(Duration::from_secs(5)))
        .await
        .expect("client should connect");
    ableton
}

/// Skips envelopes sent in the background (version check, heartbeats).
async fn envelope_where(live: &FakeLive, matches: impl Fn(&Value) -> bool) -> Value {
    loop {
        let envelope = live.next_envelope().await;
        if matches(&envelope) {
            return envelope;
        }
    }
}

fn first_command_named(name: &'static str) -> impl Fn(&Value) -> bool {
    move |envelope| envelope["commands"][0]["name"] == name
}

/// Whether any command of the envelope addresses `ns`. Background commands
/// (like the version check) may share an envelope with the ones under test.
fn has_command_for(ns: &'static str) -> impl Fn(&Value) -> bool {
    move |envelope| {
        envelope["commands"]
            .as_array()
            .is_some_and(|commands| commands.iter().any(|command| command["ns"] == ns))
    }
}

#[tokio::test]
async fn reads_and_writes_typed_properties() {
    let live = FakeLive::start(default_handler).await;
    let ableton = connect(live.options()).await;
    assert!(ableton.is_connected());

    let song = ableton.song();
    assert_eq!(song.tempo().await.unwrap(), 120.0);

    song.set_tempo(128).await.unwrap();
    let envelope = envelope_where(&live, first_command_named("set_prop")).await;
    assert_eq!(
        envelope["commands"][0],
        json!({ "ns": "song", "name": "set_prop", "args": { "prop": "tempo", "value": 128.0 } })
    );

    ableton.close().await;
    assert!(!ableton.is_connected());
}

#[tokio::test]
async fn batches_concurrent_commands_into_one_envelope() {
    let live = FakeLive::start(default_handler).await;
    let ableton = connect(live.options()).await;
    let song = ableton.song();

    let (tempo, playing) = tokio::join!(song.tempo(), song.is_playing());
    assert_eq!(tempo.unwrap(), 120.0);
    assert!(!playing.unwrap());

    let envelope = envelope_where(&live, has_command_for("song")).await;
    let props: Vec<_> = envelope["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|command| command["ns"] == "song")
        .map(|command| command["args"]["prop"].clone())
        .collect();
    assert_eq!(props, vec![json!("tempo"), json!("is_playing")]);
}

#[tokio::test]
async fn serves_unchanged_values_from_the_etag_cache() {
    let live = FakeLive::start(|command| {
        if command["args"]["prop"] != "devices" {
            return default_handler(command);
        }
        assert_eq!(command["cache"], true);
        if command["etag"] == "e1" {
            return Ok(json!({ "__cached": true }));
        }
        Ok(json!({
            "etag": "e1",
            "data": [
                { "id": "d1", "name": "Operator", "type": "instrument", "class_name": "Operator" },
                { "id": "d2", "name": "Drums", "type": "instrument", "class_name": "DrumGroupDevice" },
                { "id": "d3", "name": "Looper", "type": "audio_effect", "class_name": "Looper" }
            ]
        }))
    })
    .await;
    let ableton = connect(live.options()).await;
    let track = Track::new(
        &ableton,
        RawTrack {
            id: "t1".into(),
            ..Default::default()
        },
    );

    let first = track.devices().await.unwrap();
    let second = track.devices().await.unwrap();

    assert_eq!(first.len(), 3);
    assert_eq!(second.len(), 3);
    assert!(matches!(second[0], AnyDevice::Device(_)));
    assert!(second[1].is_rack());
    assert!(second[2].is_looper());
    assert_eq!(second[0].raw().kind, DeviceType::Instrument);

    let cached_read =
        envelope_where(&live, |envelope| envelope["commands"][0]["etag"] == "e1").await;
    assert_eq!(cached_read["commands"][0]["nsid"], "t1");
}

#[tokio::test]
async fn delivers_listener_events_until_unsubscribed() {
    let live = FakeLive::start(|command| match command["name"].as_str().unwrap() {
        "add_listener" => Ok(command["args"]["eventId"].clone()),
        "remove_listener" => Ok(json!(true)),
        _ => default_handler(command),
    })
    .await;
    let ableton = connect(live.options()).await;
    let song = ableton.song();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let subscription = song
        .on_tempo(move |tempo| {
            tx.send(tempo).unwrap();
        })
        .await
        .unwrap();

    live.push(subscription.event_id(), json!(99.5));
    let tempo = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .unwrap();
    assert_eq!(tempo, Some(99.5));

    assert!(subscription.unsubscribe().await.unwrap());
    let envelope = envelope_where(&live, first_command_named("remove_listener")).await;
    assert_eq!(envelope["commands"][0]["args"], json!({ "prop": "tempo" }));
}

#[tokio::test]
async fn reports_failed_commands_with_their_python_error_type() {
    let live = FakeLive::start(|command| match command["name"].as_str().unwrap() {
        "explode" => Err("boom".into()),
        _ => default_handler(command),
    })
    .await;
    let ableton = connect(live.options()).await;

    let error = ableton
        .song()
        .handle()
        .call::<Value>("explode", None)
        .await
        .unwrap_err();
    match error {
        Error::Command {
            message,
            error_type,
        } => {
            assert_eq!(message, "boom");
            assert_eq!(error_type.as_deref(), Some("RuntimeError"));
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[tokio::test]
async fn authenticates_with_a_password() {
    let live = FakeLive::start_with_password(Some("secret"), default_handler).await;
    let ableton = connect(AbletonOptions {
        password: Some("secret".into()),
        ..live.options()
    })
    .await;

    let auth = live.next_envelope().await;
    assert_eq!(auth["commands"].as_array().unwrap().len(), 1);
    assert_eq!(auth["commands"][0]["name"], "authenticate");
    assert_eq!(ableton.song().tempo().await.unwrap(), 120.0);
}

#[tokio::test]
async fn fails_to_start_with_a_wrong_or_missing_password() {
    let live = FakeLive::start_with_password(Some("secret"), default_handler).await;

    let wrong = Ableton::new(AbletonOptions {
        password: Some("nope".into()),
        ..live.options()
    });
    let error = wrong.start(Some(Duration::from_secs(5))).await.unwrap_err();
    assert!(matches!(error, Error::Auth(_)), "{error:?}");

    let missing = Ableton::new(live.options());
    let error = missing
        .start(Some(Duration::from_secs(5)))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Auth(_)), "{error:?}");
}

#[tokio::test]
async fn times_out_when_live_does_not_answer() {
    let live = FakeLive::start(default_handler).await;
    let ableton = connect(AbletonOptions {
        command_timeout: Duration::from_millis(200),
        ..live.options()
    })
    .await;

    *live.mute.lock().unwrap() = true;
    let error = ableton.song().tempo().await.unwrap_err();
    assert!(matches!(error, Error::Timeout { .. }), "{error:?}");
}

#[tokio::test]
async fn fails_pending_commands_and_reconnects_when_live_goes_away() {
    let live = FakeLive::start(default_handler).await;
    let ableton = connect(live.options()).await;
    let mut events = ableton.events();

    *live.mute.lock().unwrap() = true;
    let song = ableton.song();
    let pending = tokio::spawn(async move { song.tempo().await });
    envelope_where(&live, has_command_for("song")).await;

    live.disconnect();
    let error = pending.await.unwrap().unwrap_err();
    assert!(matches!(error, Error::Disconnected { .. }), "{error:?}");

    *live.mute.lock().unwrap() = false;
    let mut saw_disconnect = false;
    let reconnected = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match events.recv().await.unwrap() {
                ClientEvent::Disconnect(DisconnectKind::Realtime) => saw_disconnect = true,
                ClientEvent::Connect(ConnectKind::Realtime) => break,
                _ => {}
            }
        }
    })
    .await;
    assert!(reconnected.is_ok(), "client did not reconnect");
    assert!(saw_disconnect);
    assert_eq!(ableton.song().tempo().await.unwrap(), 120.0);
}

#[tokio::test]
async fn refuses_commands_before_start() {
    let ableton = Ableton::default();
    let error = ableton.song().tempo().await.unwrap_err();
    assert!(matches!(error, Error::NotStarted), "{error:?}");
}

#[tokio::test]
async fn start_times_out_without_live() {
    // Nothing listens on this port.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let ableton = Ableton::new(AbletonOptions {
        port,
        ..Default::default()
    });
    let error = ableton
        .start(Some(Duration::from_millis(500)))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::ConnectionTimeout), "{error:?}");
}

/// The Tauri plugin (and any multi-threaded runtime) needs these to be `Send`.
#[test]
fn futures_are_send() {
    fn assert_send<T: Send>(_: &T) {}

    let ableton = Ableton::default();
    let song = ableton.song();
    let track = Track::new(&ableton, RawTrack::default());

    assert_send(&ableton.start(None));
    assert_send(&ableton.close());
    assert_send(&song.tempo());
    assert_send(&song.set_tempo(120.0));
    assert_send(&song.on_tempo(|_| {}));
    assert_send(&track.devices());
    assert_send(&ableton.send_command(ableton_core::protocol::Command::new("song", "undo")));
}
