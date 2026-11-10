use crate::handlers::worker_handler::{HEARTBEAT_INTERVAL, run_heartbeat};
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

#[tokio::test(start_paused = true)]
async fn test_heartbeat_fails_after_configured_beats() {
    let started = Instant::now();

    let result = run_heartbeat(CancellationToken::new(), HEARTBEAT_INTERVAL, Some(3)).await;

    assert_eq!(
        result.unwrap_err().to_string(),
        "demo failure after 3 beats"
    );
    assert_eq!(started.elapsed(), Duration::from_secs(30));
}

#[tokio::test(start_paused = true)]
async fn test_heartbeat_stops_when_cancelled() {
    let shutdown = CancellationToken::new();
    shutdown.cancel();

    let result = run_heartbeat(shutdown, HEARTBEAT_INTERVAL, None).await;

    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_heartbeat_keeps_running_without_fail_setting() {
    let still_running = tokio::time::timeout(
        Duration::from_secs(100),
        run_heartbeat(CancellationToken::new(), HEARTBEAT_INTERVAL, None),
    )
    .await;

    assert!(
        still_running.is_err(),
        "heartbeat should never end by itself"
    );
}
