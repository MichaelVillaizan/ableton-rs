//! Connects to Live, prints the set and follows the tempo for ten seconds.
//!
//! ```sh
//! cargo run -p ableton-core --example basic
//! ```

use std::time::Duration;

use ableton_core::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let ableton = Ableton::new(AbletonOptions::default());
    ableton.start(Some(Duration::from_secs(10))).await?;

    let song = ableton.song();
    println!("Tempo: {} BPM", song.tempo().await?);

    for track in song.tracks().await? {
        println!("- {} ({})", track.raw().name, track.raw().kind);
        for device in track.devices().await? {
            println!("    {} [{}]", device.raw().name, device.raw().class_name);
        }
    }

    let subscription = song
        .on_tempo(|tempo| println!("Tempo changed: {tempo} BPM"))
        .await?;
    tokio::time::sleep(Duration::from_secs(10)).await;
    subscription.unsubscribe().await?;

    ableton.close().await;
    Ok(())
}
