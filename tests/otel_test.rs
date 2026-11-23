// The OpenTelemetry pack's own integration tests, in their own file so the pack never edits
// the shared integration_test.rs (see "Writing a pack" in the README).
// Both tests set the global OpenTelemetry providers, so they never run at the same time.
use excelsior::routes;
use excelsior::state::build_app_state;
use excelsior::utils::otel_config::{layer, shutdown_telemetry};
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, SdkMeterProvider};
use serial_test::serial;
use tokio::net::TcpListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Needs a collector on OTEL_EXPORTER_OTLP_ENDPOINT (localhost:4317 in .env.test):
// cargo test --test otel_test -- --ignored
#[tokio::test]
#[ignore]
#[serial]
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

// The /ping counter end to end: the whole app (with the `.layer(...)` line this pack adds
// to routes/mod.rs), real HTTP calls, and an in-memory exporter instead of the collector.
#[tokio::test]
#[serial]
async fn test_ping_requests_are_counted() {
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
    let exporter = InMemoryMetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter.clone())
        .build();
    opentelemetry::global::set_meter_provider(provider.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let app = routes::create_routes(build_app_state().await);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    for _ in 0..2 {
        let response = client
            .get(format!("{}/ping", address))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success());
    }
    // Another route: must not be counted.
    let _ = client.get(format!("{}/nothing-here", address)).send().await;

    provider.force_flush().unwrap();
    let mut total = 0;
    for resource_metrics in exporter.get_finished_metrics().unwrap() {
        for scope in resource_metrics.scope_metrics() {
            for metric in scope.metrics() {
                if metric.name() == "excelsior.ping"
                    && let AggregatedMetrics::U64(MetricData::Sum(sum)) = metric.data()
                {
                    total = sum.data_points().map(|point| point.value()).sum();
                }
            }
        }
    }
    assert_eq!(total, 2, "two /ping calls, one other route");
    provider.shutdown().unwrap();
}
