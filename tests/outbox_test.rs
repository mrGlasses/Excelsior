// The PostgreSQL outbox pack's own integration tests (OB-1 .. OB-5), in their own file so
// the pack never edits the shared integration_test.rs (see "Writing a pack" in the README).
// They need the test database from .env.test.
use axum::http::StatusCode;
use excelsior::database::connection;
use excelsior::engine::db_engine::DbPool;
use excelsior::handlers::outbox_handler::{OutboxSettings, run_outbox_worker};
use excelsior::routes;
use excelsior::state::build_app_state;
use serial_test::serial;
use sqlx::PgPool;
use std::time::Duration;
use tokio::net::TcpListener;

fn setup_test_env() {
    // In CI the workflow sets the variables; locally they come from .env.test.
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
}

// A pool on the test database, migrated (connection::start() runs the migrations).
async fn create_test_db_pool() -> PgPool {
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

// Inserts an event straight into the table, like someone using psql would.
async fn insert_outbox_row(pool: &PgPool, msg: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO outbox (event_type, payload) VALUES ('DemoEvent', jsonb_build_object('msg', $1::text)) RETURNING id",
    )
    .bind(msg)
    .fetch_one(pool)
    .await
    .expect("Failed to insert outbox row")
}

// (published?, attempts, failed?, last_error) for one event.
async fn outbox_state(pool: &PgPool, id: i64) -> (bool, i32, bool, Option<String>) {
    sqlx::query_as(
        "SELECT published_at IS NOT NULL, attempts, failed_at IS NOT NULL, last_error FROM outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("Failed to read outbox row")
}

async fn delete_outbox_rows(pool: &PgPool, ids: &[i64]) {
    sqlx::query("DELETE FROM outbox WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await
        .expect("Failed to clean up outbox rows");
}

// Waits until the event is published or `limit` passes; returns how long it took.
async fn wait_until_published(pool: &PgPool, id: i64, limit: Duration) -> Option<Duration> {
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
    let pool = create_test_db_pool().await;
    let shutdown = tokio_util::sync::CancellationToken::new();
    let token = shutdown.clone();
    let settings = OutboxSettings {
        max_attempts,
        recheck_interval,
    };
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
    let (id, event_type, payload): (i64, String, serde_json::Value) =
        sqlx::query_as("SELECT id, event_type, payload FROM outbox WHERE payload->>'msg' = $1")
            .bind(&tag)
            .fetch_one(&pool)
            .await
            .expect("the event should exist");
    assert_eq!(event_type, "DemoEvent");
    assert_eq!(payload, serde_json::json!({ "msg": tag }));

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
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox WHERE payload->>'msg' = 'rollback'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
#[serial]
async fn test_outbox_worker_wakes_up_on_commit() {
    // OB-2: the recheck is 30s, so only LISTEN/NOTIFY can make this fast.
    setup_test_env();
    let pool = create_test_db_pool().await;
    let shutdown = start_outbox_worker(10, Duration::from_secs(30)).await;
    tokio::time::sleep(Duration::from_millis(500)).await; // let it finish its first pass

    let id = insert_outbox_row(&pool, &unique_tag("ob2")).await;
    let took = wait_until_published(&pool, id, Duration::from_secs(3)).await;

    shutdown.cancel();
    delete_outbox_rows(&pool, &[id]).await;
    let took = took.expect("the event should be handled");
    assert!(took < Duration::from_secs(1), "took {:?}", took);
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
            "SELECT count(*) FROM outbox WHERE id = ANY($1) AND published_at IS NULL",
        )
        .bind(&ids)
        .fetch_one(&pool)
        .await
        .unwrap();
    }
    let handled_twice: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox WHERE id = ANY($1) AND attempts <> 1")
            .bind(&ids)
            .fetch_one(&pool)
            .await
            .unwrap();

    first.cancel();
    second.cancel();
    delete_outbox_rows(&pool, &ids).await;
    assert_eq!(remaining, 0, "all 100 events should be handled");
    assert_eq!(handled_twice, 0, "no event may be handled twice");
}
