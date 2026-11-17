// The OpenTelemetry pack's own integration test, in its own file so the pack never edits
// the shared integration_test.rs (see "Writing a pack" in the README).
use excelsior::utils::otel_config::{layer, shutdown_telemetry};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Needs a collector on OTEL_EXPORTER_OTLP_ENDPOINT (localhost:4317 in .env.test):
// cargo test --test otel_test -- --ignored
#[tokio::test]
#[ignore]
async fn test_spans_reach_the_collector() {
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
    // Same layer the app adds through pack_tracing.rs.
    let _ = tracing_subscriber::registry()
        .with(layer())
        .with(tracing_subscriber::fmt::layer())
        .try_init();

    tracing::info!("Test log message from the otel integration test");
    tracing::warn!("Warning message");

    // Give the batch exporter time, then flush what's left (what the app does at shutdown).
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    shutdown_telemetry();
}
