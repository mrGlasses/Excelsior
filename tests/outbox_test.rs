// The MariaDB outbox pack's own integration tests (OB-1 .. OB-5), in their own file so the
// pack never edits the shared integration_test.rs (see "Writing a pack" in the README).
// They need the test database from .env.test.
use axum::http::StatusCode;
use excelsior::database::connection;
use excelsior::engine::db_engine::DbPool;
use excelsior::handlers::outbox_handler::{OutboxSettings, run_outbox_worker};
use excelsior::routes;
use excelsior::state::build_app_state;
use serial_test::serial;
use sqlx::MySqlPool;
use std::time::Duration;
use tokio::net::TcpListener;

fn setup_test_env() {
    // In CI the workflow sets the variables; locally they come from .env.test.
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
}

// A pool on the test database, migrated (connection::start() runs the migrations once
// per test run).
async fn create_test_db_pool() -> MySqlPool {
    setup_test_env();
    let DbPool::Real(pool) = &*connection::start().await;
    pool.clone()
}

// Starts the whole app (every pack's state) on a free port. The workers are not started:
// the tests that need the outbox worker start their own (start_outbox_worker).
async fn spawn_app() -> String {
    setup_test_env();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = routes::create_routes(build_app_state().await);
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    format!("http://127.0.0.1:{}", port)
}

// A text no other test run uses, so tests only look at their own events.
fn unique_tag(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}-{}", prefix, nanos)
}

// Inserts an event straight into the table, like someone using a SQL client would.
async fn insert_outbox_row(pool: &MySqlPool, msg: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO outbox (event_type, payload) VALUES ('DemoEvent', JSON_OBJECT('msg', ?)) RETURNING id",
    )
    .bind(msg)
    .fetch_one(pool)
    .await
    .expect("Failed to insert outbox row")
}

// (published?, attempts, failed?, last_error) for one event.
async fn outbox_state(pool: &MySqlPool, id: i64) -> (bool, i32, bool, Option<String>) {
    let row: (i64, i32, i64, Option<String>) = sqlx::query_as(
        "SELECT CAST(published_at IS NOT NULL AS SIGNED), attempts, CAST(failed_at IS NOT NULL AS SIGNED), last_error FROM outbox WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("Failed to read outbox row");
    (row.0 != 0, row.1, row.2 != 0, row.3)
}

async fn delete_outbox_rows(pool: &MySqlPool, ids: &[i64]) {
    for id in ids {
        sqlx::query("DELETE FROM outbox WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .expect("Failed to clean up outbox rows");
    }
}

// Waits until the event is published or `limit` passes; returns how long it took.
async fn wait_until_published(pool: &MySqlPool, id: i64, limit: Duration) -> Option<Duration> {
    let started = std::time::Instant::now();
    while started.elapsed() < limit {
        if outbox_state(pool, id).await.0 {
            return Some(started.elapsed());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    None
}

// Starts an outbox worker in the background; cancel the returned token to stop it.
async fn start_outbox_worker(
    max_attempts: i32,
    recheck_interval: Duration,
) -> tokio_util::sync::CancellationToken {
    start_outbox_worker_with(OutboxSettings {
        max_attempts,
        recheck_interval,
        // No waiting between tries: these tests are about the passes, not the backoff.
        retry_base: Duration::ZERO,
    })
    .await
}

async fn start_outbox_worker_with(settings: OutboxSettings) -> tokio_util::sync::CancellationToken {
    let pool = create_test_db_pool().await;
    let shutdown = tokio_util::sync::CancellationToken::new();
    let token = shutdown.clone();
    tokio::spawn(async move { run_outbox_worker(pool, settings, token).await });
    shutdown
}

#[tokio::test]
async fn test_outbox_event_is_written_with_the_change() {
    // OB-1 step 1
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let tag = unique_tag("ob1");

    let response = reqwest::Client::new()
        .post(format!("{}/demo-events", address))
        .json(&serde_json::json!({ "msg": tag }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), StatusCode::CREATED.as_u16());
    let (id, event_type, payload): (i64, String, String) = sqlx::query_as(
        "SELECT id, event_type, CAST(payload AS CHAR) FROM outbox WHERE JSON_VALUE(payload, '$.msg') = ?",
    )
    .bind(&tag)
    .fetch_one(&pool)
    .await
    .expect("the event should exist");
    assert_eq!(event_type, "DemoEvent");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&payload).unwrap(),
        serde_json::json!({ "msg": tag })
    );

    delete_outbox_rows(&pool, &[id]).await;
}

#[tokio::test]
async fn test_outbox_event_is_rolled_back_with_the_change() {
    // OB-1 step 2
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/demo-events", address))
        .json(&serde_json::json!({ "msg": "rollback" }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        response.status().as_u16(),
        StatusCode::INTERNAL_SERVER_ERROR.as_u16()
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox WHERE JSON_VALUE(payload, '$.msg') = 'rollback'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
#[serial]
async fn test_outbox_worker_wakes_up_after_commit_in_this_app() {
    // OB-2, events from this app: the recheck is 30s, so only the in-process wake-up
    // (wake_outbox_worker after commit) can make this fast.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let shutdown = start_outbox_worker(10, Duration::from_secs(30)).await;
    tokio::time::sleep(Duration::from_millis(500)).await; // let it finish its first pass

    let tag = unique_tag("ob2-app");
    let started = std::time::Instant::now();
    reqwest::Client::new()
        .post(format!("{}/demo-events", address))
        .json(&serde_json::json!({ "msg": tag }))
        .send()
        .await
        .expect("Failed to execute request.");
    let id: i64 =
        sqlx::query_scalar("SELECT id FROM outbox WHERE JSON_VALUE(payload, '$.msg') = ?")
            .bind(&tag)
            .fetch_one(&pool)
            .await
            .expect("the event should exist");
    let published = wait_until_published(&pool, id, Duration::from_secs(3)).await;

    shutdown.cancel();
    delete_outbox_rows(&pool, &[id]).await;
    assert!(published.is_some(), "the event should be handled");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
#[serial]
async fn test_outbox_worker_finds_outside_inserts_within_a_second() {
    // OB-2, events from outside the app (SQL client): found by the 1-second recheck.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let shutdown = start_outbox_worker(10, Duration::from_secs(1)).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let id = insert_outbox_row(&pool, &unique_tag("ob2-sql")).await;
    let took = wait_until_published(&pool, id, Duration::from_secs(3)).await;

    shutdown.cancel();
    delete_outbox_rows(&pool, &[id]).await;
    let took = took.expect("the event should be handled");
    // At most one recheck interval, plus a little time to handle it.
    assert!(took < Duration::from_millis(1500), "took {:?}", took);
}

#[tokio::test]
#[serial]
async fn test_outbox_worker_catches_up_after_downtime() {
    // OB-3: the event commits while no worker runs; the worker's first pass takes it.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let id = insert_outbox_row(&pool, &unique_tag("ob3")).await;

    let shutdown = start_outbox_worker(10, Duration::from_secs(30)).await;
    let took = wait_until_published(&pool, id, Duration::from_secs(5)).await;

    shutdown.cancel();
    delete_outbox_rows(&pool, &[id]).await;
    assert!(
        took.is_some(),
        "the waiting event should be handled at startup"
    );
}

#[tokio::test]
#[serial]
async fn test_outbox_failing_event_is_retried_then_set_aside() {
    // OB-4: "boom" always fails; 3 attempts, then it's set aside without blocking others.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let shutdown = start_outbox_worker(3, Duration::from_millis(100)).await;

    let boom = insert_outbox_row(&pool, "boom").await;
    let after = insert_outbox_row(&pool, &unique_tag("ob4-after")).await;
    let after_took = wait_until_published(&pool, after, Duration::from_secs(3)).await;

    let started = std::time::Instant::now();
    let mut state = outbox_state(&pool, boom).await;
    while !state.2 && started.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(50)).await;
        state = outbox_state(&pool, boom).await;
    }

    shutdown.cancel();
    delete_outbox_rows(&pool, &[boom, after]).await;
    assert!(
        after_took.is_some(),
        "a failing event must not block the next one"
    );
    let (published, attempts, failed, last_error) = state;
    assert!(!published);
    assert!(failed, "the event should be set aside");
    assert_eq!(attempts, 3);
    assert_eq!(last_error.as_deref(), Some("demo failure"));
}

#[tokio::test]
#[serial]
async fn test_outbox_failing_event_waits_longer_each_time() {
    // HFEX0004: with a 1s base, tries come at ~0s, ~1s, ~3s (1s, then 2s of waiting).
    // Without the backoff, a 100ms re-check would have tried it ~25 times by now.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let shutdown = start_outbox_worker_with(OutboxSettings {
        max_attempts: 10,
        recheck_interval: Duration::from_millis(100),
        retry_base: Duration::from_secs(1),
    })
    .await;

    let boom = insert_outbox_row(&pool, "boom").await;
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let (_, attempts_at_2_5s, failed, _) = outbox_state(&pool, boom).await;
    let waiting: i64 = sqlx::query_scalar(
        "SELECT CAST(next_attempt_at > NOW(6) AS SIGNED) FROM outbox WHERE id = ?",
    )
    .bind(boom)
    .fetch_one(&pool)
    .await
    .unwrap();

    shutdown.cancel();
    delete_outbox_rows(&pool, &[boom]).await;
    assert_eq!(attempts_at_2_5s, 2, "tried at ~0s and ~1s, next at ~3s");
    assert!(!failed);
    assert_eq!(waiting, 1, "next_attempt_at should be in the future");
}

#[tokio::test]
#[serial]
async fn test_outbox_two_workers_never_share_an_event() {
    // OB-5: two workers, 100 events: every event handled exactly once
    // (attempts would be 2 if both workers had handled the same event).
    setup_test_env();
    let pool = create_test_db_pool().await;
    let first = start_outbox_worker(10, Duration::from_millis(200)).await;
    let second = start_outbox_worker(10, Duration::from_millis(200)).await;

    let tag = unique_tag("ob5");
    let mut ids = Vec::new();
    for n in 0..100 {
        ids.push(insert_outbox_row(&pool, &format!("{}-{}", tag, n)).await);
    }
    let started = std::time::Instant::now();
    let mut remaining: i64 = 100;
    while remaining > 0 && started.elapsed() < Duration::from_secs(15) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        remaining = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbox WHERE JSON_VALUE(payload, '$.msg') LIKE CONCAT(?, '-%') AND published_at IS NULL",
        )
        .bind(&tag)
        .fetch_one(&pool)
        .await
        .unwrap();
    }
    let handled_twice: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM outbox WHERE JSON_VALUE(payload, '$.msg') LIKE CONCAT(?, '-%') AND attempts <> 1",
    )
    .bind(&tag)
    .fetch_one(&pool)
    .await
    .unwrap();

    first.cancel();
    second.cancel();
    delete_outbox_rows(&pool, &ids).await;
    assert_eq!(remaining, 0, "all 100 events should be handled");
    assert_eq!(handled_twice, 0, "no event may be handled twice");
}
