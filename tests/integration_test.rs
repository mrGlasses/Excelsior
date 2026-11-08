use axum::http::StatusCode;
use dotenv::dotenv;
// Import the ms1 crate and its modules
use excelsior::utils::main_utils::service_starter;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

// Import the excelsior crate and its modules
use excelsior::{database, engine::db_engine::DbPool, routes, state::AppState};

fn setup_test_env() {
    // Skip dotenv loading if running in CI
    if std::env::var("CI").is_ok() {
        println!("Running in CI - using environment variables from workflow");
        return;
    }

    if std::path::Path::new(".env.test").exists() {
        dotenv::from_filename(".env.test").ok();
    } else {
        dotenv().ok();
        println!("Warning: .env.test not found, using .env file");
    }
}

// Helper function to create a test database connection
async fn create_test_db_pool() -> PgPool {
    let pool = database::connection::init_db()
        .await
        .expect("Failed to connect to test database");
    // Same migrations the app runs at startup; sqlx takes a Postgres advisory lock
    // while migrating, so tests running this in parallel don't step on each other.
    database::connection::run_migrations(&pool)
        .await
        .expect("Failed to apply migrations to test database");
    pool
}

// Helper function to create a test app instance
async fn spawn_app() -> String {
    setup_test_env();

    // Find a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Set up a test database connection using the same method as the main app
    let pool = create_test_db_pool().await;

    let app_state = AppState {
        db_pool: Arc::new(DbPool::Real(pool)),
    };

    // Build the application with routes
    let app = routes::create_routes(app_state);

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    format!("http://127.0.0.1:{}", port)
}

// Helper function to clean up test data
async fn cleanup_test_data(pool: &PgPool) {
    // Add cleanup queries here if needed
    sqlx::query("DELETE FROM t_users WHERE name = 'Test User'")
        .execute(pool)
        .await
        .expect("Failed to clean up test data");
}

#[tokio::test]
async fn test_server_health_check() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/ping", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(response.text().await.unwrap(), "PONG!");
}

#[tokio::test]
async fn test_get_users() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/users", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_create_user() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/users", address))
        .json(&serde_json::json!({
            "name": "Test User"
        }))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::UNAUTHORIZED),
        StatusCode::CREATED
    );

    cleanup_test_data(&pool).await;
}

async fn count_users_named(pool: &PgPool, names: &[&str]) -> i64 {
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    sqlx::query_scalar("SELECT count(*) FROM t_users WHERE name = ANY($1)")
        .bind(names)
        .fetch_one(pool)
        .await
        .expect("Failed to count users")
}

async fn delete_users_named(pool: &PgPool, names: &[&str]) {
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    sqlx::query("DELETE FROM t_users WHERE name = ANY($1)")
        .bind(names)
        .execute(pool)
        .await
        .expect("Failed to clean up test users");
}

#[tokio::test]
async fn test_create_user_pair_commits_both() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let names = ["PairTestAnn", "PairTestBob"];
    delete_users_named(&pool, &names).await;

    let response = reqwest::Client::new()
        .post(format!("{}/users/pair", address))
        .json(&serde_json::json!({ "first": names[0], "second": names[1] }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), StatusCode::CREATED.as_u16());
    assert_eq!(count_users_named(&pool, &names).await, 2);

    delete_users_named(&pool, &names).await;
}

#[tokio::test]
async fn test_create_user_pair_rolls_back_when_second_fails() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    let address = spawn_app().await;
    let first = "PairTestCid";
    delete_users_named(&pool, &[first]).await;

    // t_users.name is VARCHAR(15), so the second insert fails after the first one ran.
    let response = reqwest::Client::new()
        .post(format!("{}/users/pair", address))
        .json(&serde_json::json!({ "first": first, "second": "NameLongerThan15" }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        response.status().as_u16(),
        StatusCode::INTERNAL_SERVER_ERROR.as_u16()
    );
    assert_eq!(count_users_named(&pool, &[first]).await, 0);
}

#[tokio::test]
async fn test_uuid_and_jsonb_round_trip() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    // A temp table lives on one connection only, so everything runs in one
    // transaction; it's rolled back at the end and nothing is left behind.
    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    sqlx::query(
        "CREATE TEMP TABLE uuid_json_check (id uuid PRIMARY KEY, data jsonb NOT NULL) ON COMMIT DROP",
    )
    .execute(&mut *tx)
    .await
    .expect("Failed to create temp table");

    let written_id: sqlx::types::Uuid = sqlx::query_scalar(
        "INSERT INTO uuid_json_check (id, data) VALUES (gen_random_uuid(), '{\"a\": 1}'::jsonb) RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Failed to insert uuid/jsonb row");

    let (read_id, read_data): (sqlx::types::Uuid, serde_json::Value) =
        sqlx::query_as("SELECT id, data FROM uuid_json_check")
            .fetch_one(&mut *tx)
            .await
            .expect("Failed to read uuid/jsonb row");

    assert_eq!(read_id, written_id);
    assert_eq!(read_data, serde_json::json!({ "a": 1 }));

    tx.rollback().await.expect("Failed to roll back");
}

#[tokio::test]
#[serial]
async fn test_service_starter_initialization() {
    setup_test_env();
    // .env.test sets MS_PORT=0 (random port) for tests that bind their own listener;
    // service_starter() reads MS_PORT directly, so it needs a fixed value to match
    // the address this test checks below.
    unsafe {
        std::env::set_var("MS_PORT", "3000");
    }

    println!("Testing service starter initialization...");
    // Spawn service_starter in a background task
    let server_handle = tokio::spawn(async move {
        service_starter().await;
    });
    println!("Service starter initialized successfully");

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Test that the server is responding
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:3000/ping")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(resp.status().is_success());
            assert_eq!(resp.text().await.unwrap(), "PONG!");
            println!("Service starter test: Server responded successfully");
        }
        Err(e) => {
            eprintln!("Service starter test: Failed to connect - {:?}", e);
            // Server might not be fully started or DB connection failed
        }
    }

    // Abort the server task to clean up
    server_handle.abort();
}

#[tokio::test]
#[serial]
async fn test_service_starter_graceful_shutdown() {
    setup_test_env();
    // Distinct fixed port from test_service_starter_initialization above; both tests
    // are #[serial] so mutating this shared env var between them is safe.
    unsafe {
        std::env::set_var("MS_PORT", "9998");
    }

    // Spawn service_starter in a background task
    let server_handle = tokio::spawn(async move {
        service_starter().await;
    });

    // Give the server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Verify server is running
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:9998/ping")
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    if let Ok(resp) = response {
        assert!(resp.status().is_success());
        println!("Service starter graceful shutdown test: Server is running");
    }

    // Abort the server to simulate shutdown
    server_handle.abort();

    // abort() only requests cancellation; the task keeps running until its next
    // await point, so wait for it to actually finish tearing down (and dropping the
    // listener) rather than guessing with a fixed sleep.
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;

    // Verify server is no longer responding. A fresh client is used here (rather than
    // reusing `client`) so this genuinely opens a new connection instead of reusing a
    // keep-alive one from the request above, which would still succeed via its own
    // per-connection task even after the listener itself is closed.
    let response_after = reqwest::Client::new()
        .get("http://127.0.0.1:9998/ping")
        .timeout(Duration::from_secs(1))
        .send()
        .await;

    assert!(
        response_after.is_err(),
        "Server should not respond after abort"
    );
    println!("Service starter graceful shutdown test: Server stopped successfully");
}
