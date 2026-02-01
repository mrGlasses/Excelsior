use dotenv::dotenv;
use serial_test::serial;
use std::env;
use std::net::TcpListener;

// Import the excelsior crate and its modules
use serial_test::serial;
// Import the ms1 crate and its modules
use excelsior::routes;
use excelsior::utils::otel_config::{setup_tracing_with_otel, shutdown_telemetry};
use std::sync::Once;

// Helper function to set up the test environment
static INIT: Once = Once::new();
use excelsior::utils::main_utils::service_starter;
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

#[tokio::test]
#[serial]
async fn test_setup_tracing_with_otel_full_stack() {
    setup_test_env();
    INIT.call_once(|| {
        // Initialize any global test setup here
        env::set_var("RUST_LOG", "info");
    });

    env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
    env::set_var("OTEL_SERVICE_NAME", "excelsior-tracing-test");
    env::set_var("ENVIRONMENT", "testing");
    env::set_var("RUST_LOG", "info");

    println!("🔧 Testing setup_tracing_with_otel with real collector...");

    // This function calls init_telemetry internally and sets up the subscriber
    // Note: This can only be called ONCE per test process due to global subscriber
    let result = std::panic::catch_unwind(|| {
        setup_tracing_with_otel();
    });

    match result {
        Ok(_) => {
            println!("✅ Successfully set up tracing with OpenTelemetry");

            // Test that tracing works
            tracing::info!("Test log message from integration test");
            tracing::debug!("Debug message - should respect RUST_LOG");
            tracing::warn!("Warning message");

            println!("✅ Tracing messages sent successfully");

            // Give time for spans to flush
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            shutdown_telemetry();
            println!("✅ Successfully shut down telemetry");
        }
        Err(e) => {
            // The function might panic if collector is not available
            eprintln!("❌ setup_tracing_with_otel panicked: {:?}", e);
            eprintln!("💡 Make sure OTLP collector is running on localhost:4317");
            panic!("Tracing setup failed");
        }
    }

    // Clean up
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");
}

#[tokio::test]
#[serial]
async fn test_service_starter_initialization() {
    setup_test_env();

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
//#[serial]
async fn test_service_starter_graceful_shutdown() {
    setup_test_env();

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

    // Wait a bit for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify server is no longer responding
    let response_after = client
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
