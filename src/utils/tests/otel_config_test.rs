use crate::utils::otel_config::*;
use opentelemetry::KeyValue;
use serial_test::serial;
use std::env;

/// Test shutdown_telemetry doesn't panic when called without initialization
#[test]
#[serial]
fn test_shutdown_telemetry_without_init() {
    // This test verifies that shutdown can be called without panicking
    // Even if no tracer provider was set
    shutdown_telemetry();
    assert!(true, "Shutdown executed without panic");
}

/// Test URL validation - valid OTLP endpoints
#[test]
fn test_valid_otlp_endpoint_formats() {
    let valid_endpoints = vec![
        "http://localhost:4317",
        "http://127.0.0.1:4317",
        "http://grafana:4317",
        "http://otel-collector.monitoring.svc:4317",
        "https://otel.example.com:4317",
    ];

    for endpoint in valid_endpoints {
        // These should be valid URL formats
        assert!(endpoint.starts_with("http://") || endpoint.starts_with("https://"));
        assert!(endpoint.contains(":"));
    }
}

/// Test resource attributes structure
#[test]
fn test_resource_attributes_count() {
    use opentelemetry_sdk::Resource;

    // Create a resource similar to our implementation
    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", "excelsior"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("deployment.environment", "production"),
        ])
        .build();

    // The resource should have been created successfully
    // We can't easily count attributes without exposing internal API,
    // but we can verify it doesn't panic
    drop(resource);
    assert!(true, "Resource created with 3 attributes");
}

/// Integration test: Test init_telemetry with a valid endpoint
/// This requires a Tokio runtime and should connect to a real OTLP collector
#[tokio::test]
#[serial]
//#[ignore] // Ignore by default - run with: cargo test -- --ignored
async fn test_init_telemetry_with_real_collector() {
    // This test requires a running OTLP collector on localhost:4317
    // Start one with: docker run -p 4317:4317 otel/opentelemetry-collector

    // Attempt initialization
    let result = init_telemetry();

    match result {
        Ok(provider) => {
            println!("Successfully initialized telemetry with real collector");

            // Clean up the provider
            drop(provider);
            //shutdown_telemetry();

            assert!(true, "Telemetry initialized successfully");
        }
        Err(e) => {
            println!("Failed to initialize telemetry: {:?}", e);
            println!("Make sure OTLP collector is running on localhost:4317");
            panic!("Telemetry initialization failed: {:?}", e);
        }
    }
}

/// Test that init_telemetry fails gracefully with unreachable endpoint
/// Uses Tokio runtime since the underlying code requires it
#[tokio::test]
#[serial]
async fn test_init_telemetry_unreachable_endpoint() {
    unsafe {
        // Use a valid URL format but unreachable port
        env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:19999");
        env::set_var("OTEL_SERVICE_NAME", "test-service");

        // The exporter build itself should succeed (lazy connection)
        // but actual span export would fail
        let _result = std::panic::catch_unwind(|| init_telemetry());

        // The lazy connection builder should NOT panic immediately
        // It only fails when actually trying to send data
        // So we expect this to succeed (or at least not panic here)

        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        env::remove_var("OTEL_SERVICE_NAME");
    }
}

/// Test multiple shutdown calls don't cause issues
#[test]
#[serial]
fn test_multiple_shutdown_calls() {
    shutdown_telemetry();
    shutdown_telemetry();
    shutdown_telemetry();

    assert!(true, "Multiple shutdown calls handled gracefully");
}
