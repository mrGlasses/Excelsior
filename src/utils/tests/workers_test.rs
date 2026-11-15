use crate::handlers::worker_handler::stuck_loop;
use crate::utils::workers::{DEFAULT_SHUTDOWN_TIMEOUT_SECS, Workers, shutdown_timeout_from};
use anyhow::anyhow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

// A well-behaved worker: waits for the stop signal, then returns.
async fn polite_worker(shutdown: CancellationToken) -> anyhow::Result<()> {
    shutdown.cancelled().await;
    Ok(())
}

#[tokio::test]
async fn test_worker_stops_on_shutdown() {
    let shutdown = CancellationToken::new();
    let mut workers = Workers::new(shutdown.clone(), Duration::from_secs(8));
    workers.spawn("polite", polite_worker);

    shutdown.cancel();

    assert!(!workers.run_until_shutdown().await);
}

#[tokio::test]
async fn test_failing_worker_stops_the_others() {
    let shutdown = CancellationToken::new();
    let other_stopped = Arc::new(AtomicBool::new(false));
    let flag = other_stopped.clone();

    let mut workers = Workers::new(shutdown.clone(), Duration::from_secs(8));
    workers.spawn("other", move |token: CancellationToken| async move {
        token.cancelled().await;
        flag.store(true, Ordering::SeqCst);
        Ok(())
    });
    workers.spawn("broken", |_token| async { Err(anyhow!("demo failure")) });

    // Nobody cancels the token here: the failing worker has to do it.
    assert!(workers.run_until_shutdown().await);
    assert!(shutdown.is_cancelled());
    assert!(other_stopped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_panicking_worker_counts_as_failure() {
    let shutdown = CancellationToken::new();
    let mut workers = Workers::new(shutdown.clone(), Duration::from_secs(8));
    workers.spawn("panicky", |_token| async {
        panic!("boom");
    });

    assert!(workers.run_until_shutdown().await);
    assert!(shutdown.is_cancelled());
}

#[tokio::test]
async fn test_worker_finishing_early_is_not_a_failure() {
    let shutdown = CancellationToken::new();
    let mut workers = Workers::new(shutdown.clone(), Duration::from_secs(8));
    workers.spawn("one-shot", |_token| async { Ok(()) });

    let canceller = shutdown.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        canceller.cancel();
    });

    assert!(!workers.run_until_shutdown().await);
}

#[tokio::test(start_paused = true)]
async fn test_stuck_worker_is_abandoned_after_timeout() {
    let shutdown = CancellationToken::new();
    let mut workers = Workers::new(shutdown.clone(), Duration::from_secs(8));
    workers.spawn("stuck", stuck_loop);
    workers.spawn("polite", polite_worker);

    shutdown.cancel();
    let started = Instant::now();

    // Ignoring the stop signal is not a failure: shutdown just carries on without it.
    assert!(!workers.run_until_shutdown().await);
    assert_eq!(started.elapsed(), Duration::from_secs(8));
}

#[test]
fn test_shutdown_timeout_unset_uses_default() {
    assert_eq!(
        shutdown_timeout_from(None),
        Duration::from_secs(DEFAULT_SHUTDOWN_TIMEOUT_SECS)
    );
    assert_eq!(DEFAULT_SHUTDOWN_TIMEOUT_SECS, 8);
}

#[test]
fn test_shutdown_timeout_from_env_value() {
    assert_eq!(
        shutdown_timeout_from(Some(" 3 ".to_string())),
        Duration::from_secs(3)
    );
}

#[test]
#[should_panic(expected = "WORKER_SHUTDOWN_TIMEOUT_SECS must be a number of seconds.")]
fn test_shutdown_timeout_not_a_number_panics() {
    shutdown_timeout_from(Some("eight".to_string()));
}
