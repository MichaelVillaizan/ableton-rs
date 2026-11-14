//! Measures how fast Live pushes listener events. Port of
//! `ableton-js/scripts/test-message-throughput.ts`.
//!
//! Listens to the output meters of every non-MIDI track until Ctrl+C, then
//! prints the throughput, the spacing between events and the end-to-end
//! latency (receive time minus the `ts` the Remote Script stamps on each
//! frame; Live and this program share a clock). Play something in Live so the
//! meters move.
//!
//! ```sh
//! cargo run -p ableton-core --example message_throughput
//! ```

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ableton_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let messages: Arc<Mutex<Vec<Instant>>> = Arc::default();
    let latencies: Arc<Mutex<Vec<f64>>> = Arc::default();

    let ableton = Ableton::new(AbletonOptions::default());

    let mut events = ableton.events();
    let event_latencies = Arc::clone(&latencies);
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(ClientEvent::Connect(kind)) => println!("Live connected ({kind:?})"),
                Ok(ClientEvent::Disconnect(kind)) => println!("Live disconnected ({kind:?})"),
                Ok(ClientEvent::Message(message)) => {
                    if message["event"] != "result"
                        && let Some(ts) = message["ts"].as_f64()
                    {
                        event_latencies.lock().unwrap().push(now_ms() - ts);
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("Event receiver lagged, {skipped} events not measured");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    println!("Waiting for Live to start...");
    ableton.start(None).await?;
    println!("Connection to Live established");

    let tracks = ableton.song().tracks().await?;
    println!("Got tracks");

    let mut subscriptions = Vec::new();
    for track in tracks {
        if track.raw().kind == TrackType::Midi {
            continue;
        }
        let left = Arc::clone(&messages);
        subscriptions.push(
            track
                .on_output_meter_left(move |_| left.lock().unwrap().push(Instant::now()))
                .await?,
        );
        let right = Arc::clone(&messages);
        subscriptions.push(
            track
                .on_output_meter_right(move |_| right.lock().unwrap().push(Instant::now()))
                .await?,
        );
    }

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");

    for subscription in subscriptions {
        let _ = subscription.unsubscribe().await;
    }
    ableton.close().await;

    summarize(
        &messages.lock().unwrap(),
        &mut latencies.lock().unwrap().clone(),
    );
    Ok(())
}

fn summarize(messages: &[Instant], latencies: &mut [f64]) {
    if messages.len() < 2 {
        println!(
            "\nOnly captured {} message(s), nothing to summarize.",
            messages.len()
        );
        return;
    }

    let mut deltas: Vec<f64> = messages
        .windows(2)
        .map(|pair| ms(pair[1] - pair[0]))
        .collect();
    deltas.sort_by(f64::total_cmp);

    let avg = deltas.iter().sum::<f64>() / deltas.len() as f64;
    let total_duration = ms(messages[messages.len() - 1] - messages[0]);

    println!("\n--- Message timing summary ---");
    println!("Messages received: {}", messages.len());
    println!("Total duration: {total_duration:.2} ms");
    println!(
        "Throughput: {:.1} msg/s",
        messages.len() as f64 / (total_duration / 1000.0)
    );
    println!("Delta between consecutive messages (ms):");
    println!("  min: {:.2}", deltas[0]);
    println!("  avg: {avg:.2}");
    println!("  p50: {:.2}", percentile(&deltas, 50.0));
    println!("  p95: {:.2}", percentile(&deltas, 95.0));
    println!("  p99: {:.2}", percentile(&deltas, 99.0));
    println!("  max: {:.2}", deltas[deltas.len() - 1]);

    if !latencies.is_empty() {
        latencies.sort_by(f64::total_cmp);
        let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
        println!("End-to-end latency, Live -> client (ms):");
        println!("  min: {:.0}", latencies[0]);
        println!("  avg: {avg:.2}");
        println!("  p50: {:.0}", percentile(latencies, 50.0));
        println!("  p95: {:.0}", percentile(latencies, 95.0));
        println!("  p99: {:.0}", percentile(latencies, 99.0));
        println!("  max: {:.0}", latencies[latencies.len() - 1]);
        println!(
            "  over 500ms: {} of {}",
            latencies.iter().filter(|&&l| l > 500.0).count(),
            latencies.len()
        );
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let index = ((p / 100.0) * sorted.len() as f64).floor() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        * 1000.0
}
