use dotenv::dotenv;
use serial_test::serial;
use std::env;
use std::net::TcpListener;

// Import the excelsior crate and its modules
use excelsior::routes;
use excelsior::utils::otel_config::{setup_tracing_with_otel, shutdown_telemetry};
use std::sync::Once;

// Helper function to set up the test environment
static INIT: Once = Once::new();

// Helper function to set up the test environment
fn setup_test_env() {
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
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    // Build the application with routes
    let app = routes::create_routes();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
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
