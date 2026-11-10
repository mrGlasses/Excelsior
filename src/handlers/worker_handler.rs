// Demo workers for utils::workers. Handlers handle routes; these handle ticks.
// Delete this file (and its lines in service_starter) when your app has real workers.
use anyhow::anyhow;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

// Logs "heartbeat" every 10 seconds until told to stop.
// DEMO_HEARTBEAT_FAIL_AFTER=3 makes it fail after the 3rd beat (to see the app exit with code 1).
pub async fn heartbeat_loop(shutdown: CancellationToken) -> anyhow::Result<()> {
    let fail_after = std::env::var("DEMO_HEARTBEAT_FAIL_AFTER").ok().map(|text| {
        text.trim()
            .parse::<u32>()
            .expect("DEMO_HEARTBEAT_FAIL_AFTER must be a number.")
    });
    run_heartbeat(shutdown, HEARTBEAT_INTERVAL, fail_after).await
}

// The loop itself, with the settings passed in so tests don't need env vars.
pub async fn run_heartbeat(
    shutdown: CancellationToken,
    interval: Duration,
    fail_after: Option<u32>,
) -> anyhow::Result<()> {
    let mut beats = 0;
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => return Ok(()),
            _ = tokio::time::sleep(interval) => {
                beats += 1;
                info!("heartbeat");
                if Some(beats) == fail_after {
                    return Err(anyhow!("demo failure after {} beats", beats));
                }
            }
        }
    }
}

// A badly behaved worker: it never looks at the token, so it never stops by itself.
// Only started with DEMO_STUCK_WORKER=true, to see the shutdown timeout working.
pub async fn stuck_loop(_shutdown: CancellationToken) -> anyhow::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}
