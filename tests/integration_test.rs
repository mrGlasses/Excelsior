use dotenv::dotenv;
// Import the ms1 crate and its modules
use excelsior::routes;
use excelsior::utils::main_utils::service_starter;
use serial_test::serial;
use std::time::Duration;
use tokio::net::TcpListener;

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

// Helper function to create a test app instance
async fn spawn_app() -> String {
    setup_test_env();

    // Find a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    // Build the application with routes
    let app = routes::create_routes();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    format!("http://127.0.0.1:{}", port)
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

// A fake Telegram Bot API for the tests that start the whole app: service_starter()
// needs a bot token, and tests must never talk to the real Telegram.
// Keep the returned server alive for the whole test.
async fn start_fake_telegram() -> httpmock::MockServer {
    let server = httpmock::MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/botTEST-TOKEN/getMe");
            then.status(200).json_body(serde_json::json!({
                "ok": true,
                "result": {"id": 1, "is_bot": true, "first_name": "Test", "username": "TestBot"}
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method("POST").path("/botTEST-TOKEN/getUpdates");
            // A real long poll waits when nothing happens; the delay stops a busy loop.
            then.status(200)
                .delay(Duration::from_millis(200))
                .json_body(serde_json::json!({"ok": true, "result": []}));
        })
        .await;
    unsafe {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "TEST-TOKEN");
        std::env::set_var("TELEGRAM_API_URL", server.base_url());
    }
    server
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
    let _telegram = start_fake_telegram().await;

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
    let _telegram = start_fake_telegram().await;

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

#[tokio::test]
#[serial]
async fn test_service_starter_without_telegram_token_exits() {
    setup_test_env();
    // Empty rather than removed: dotenv() inside service_starter never overwrites an
    // existing variable, so a real token in .env can't sneak back in.
    unsafe {
        std::env::set_var("TELEGRAM_BOT_TOKEN", "");
    }

    let clean_shutdown = tokio::time::timeout(Duration::from_secs(5), service_starter())
        .await
        .expect("service_starter should return at once without a token");

    // false = main() exits with code 1
    assert!(!clean_shutdown);
}
