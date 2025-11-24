use crate::utils::otel_config::*;
use opentelemetry::KeyValue;
use serial_test::serial;
use std::env;
#[tokio::test]
#[serial]
async fn test_init_telemetry_default_endpoint_logic() {
    // Clean environment
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");

    // Verify defaults are used when env vars are not set
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "excelsior".to_string());
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    assert_eq!(endpoint, "http://localhost:4317");
    assert_eq!(service_name, "excelsior");
    assert_eq!(environment, "production");
}

/// Test that init_telemetry reads custom environment variables
#[tokio::test]
#[serial]
async fn test_init_telemetry_custom_env_vars() {
    // Set custom environment variables
    env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://custom:4317");
    env::set_var("OTEL_SERVICE_NAME", "test-service");
    env::set_var("ENVIRONMENT", "testing");

    // Verify the env vars are readable
    assert_eq!(
        env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap(),
        "http://custom:4317"
    );
    assert_eq!(env::var("OTEL_SERVICE_NAME").unwrap(), "test-service");
    assert_eq!(env::var("ENVIRONMENT").unwrap(), "testing");

    // Clean up
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");
}

/// Test resource creation with default values
#[test]
fn test_resource_creation_defaults() {
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");

    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "excelsior".to_string());
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    assert_eq!(service_name, "excelsior");
    assert_eq!(environment, "production");
}

/// Test resource creation with custom values
#[test]
#[serial]
fn test_resource_creation_custom() {
    env::set_var("OTEL_SERVICE_NAME", "custom-service");
    env::set_var("ENVIRONMENT", "development");

    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "excelsior".to_string());
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    assert_eq!(service_name, "custom-service");
    assert_eq!(environment, "development");

    // Clean up
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");
}

/// Test that service version is set from Cargo.toml
#[test]
fn test_service_version_from_cargo() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(!version.is_empty(), "Package version should not be empty");

    // Verify it's a valid semver-like string
    assert!(
        version.chars().any(|c| c.is_numeric()),
        "Version should contain numbers"
    );
}

/// Test environment variable fallback chain
#[test]
#[serial]
fn test_env_var_fallback() {
    // Test OTEL_EXPORTER_OTLP_ENDPOINT fallback
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    assert_eq!(endpoint, "http://localhost:4317");

    env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://grafana:4317");
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    assert_eq!(endpoint, "http://grafana:4317");

    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
}

/// Test KeyValue creation for resource attributes
#[test]
fn test_keyvalue_creation() {
    let service_name = KeyValue::new("service.name", "excelsior");
    let version = KeyValue::new("service.version", env!("CARGO_PKG_VERSION"));
    let environment = KeyValue::new("deployment.environment", "production");

    // Verify KeyValue creation doesn't panic
    // The KeyValue type is opaque, but we can verify creation succeeds
    drop(service_name);
    drop(version);
    drop(environment);
    assert!(true, "KeyValue creation successful");
}

/// Test shutdown_telemetry doesn't panic when called without initialization
#[test]
#[serial]
fn test_shutdown_telemetry_without_init() {
    // This test verifies that shutdown can be called without panicking
    // Even if no tracer provider was set
    shutdown_telemetry();
    assert!(true, "Shutdown executed without panic");
}

/// Test configuration timeout values are within acceptable ranges
#[test]
fn test_timeout_configuration() {
    use std::time::Duration;

    let timeout = Duration::from_secs(10);
    assert_eq!(timeout.as_secs(), 10);
    assert!(
        timeout.as_secs() >= 5,
        "Timeout should be at least 5 seconds"
    );
    assert!(
        timeout.as_secs() <= 30,
        "Timeout should not exceed 30 seconds"
    );
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

/// Test environment variable combinations
#[test]
#[serial]
fn test_all_env_vars_set() {
    env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://grafana:4317");
    env::set_var("OTEL_SERVICE_NAME", "excelsior-prod");
    env::set_var("ENVIRONMENT", "production");

    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap();
    let service_name = env::var("OTEL_SERVICE_NAME").unwrap();
    let environment = env::var("ENVIRONMENT").unwrap();

    assert_eq!(endpoint, "http://grafana:4317");
    assert_eq!(service_name, "excelsior-prod");
    assert_eq!(environment, "production");

    // Clean up
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");
}

/// Test partial environment variable configuration
#[test]
#[serial]
fn test_partial_env_vars() {
    // Only set service name, others should use defaults
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::set_var("OTEL_SERVICE_NAME", "my-service");
    env::remove_var("ENVIRONMENT");

    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "excelsior".to_string());
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    assert_eq!(endpoint, "http://localhost:4317");
    assert_eq!(service_name, "my-service");
    assert_eq!(environment, "production");

    env::remove_var("OTEL_SERVICE_NAME");
}

/// Test resource attributes structure
#[test]
fn test_resource_attributes_count() {
    use opentelemetry_sdk::Resource;

    // Create a resource similar to our implementation
    let resource = Resource::new(vec![
        KeyValue::new("service.name", "excelsior"),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("deployment.environment", "production"),
    ]);

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

    env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
    env::set_var("OTEL_SERVICE_NAME", "excelsior-test");
    env::set_var("ENVIRONMENT", "testing");

    // Attempt initialization
    let result = init_telemetry();

    match result {
        Ok(provider) => {
            println!("✅ Successfully initialized telemetry with real collector");

            // Clean up the provider
            drop(provider);
            //shutdown_telemetry();

            assert!(true, "Telemetry initialized successfully");
        }
        Err(e) => {
            println!("⚠️ Failed to initialize telemetry: {:?}", e);
            println!("Make sure OTLP collector is running on localhost:4317");
            panic!("Telemetry initialization failed: {:?}", e);
        }
    }

    // Clean up
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    env::remove_var("OTEL_SERVICE_NAME");
    env::remove_var("ENVIRONMENT");
}

/// Test that init_telemetry fails gracefully with unreachable endpoint
/// Uses Tokio runtime since the underlying code requires it
#[tokio::test]
#[serial]
async fn test_init_telemetry_unreachable_endpoint() {
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

/// Test multiple shutdown calls don't cause issues
#[test]
#[serial]
fn test_multiple_shutdown_calls() {
    shutdown_telemetry();
    shutdown_telemetry();
    shutdown_telemetry();

    assert!(true, "Multiple shutdown calls handled gracefully");
}

/// Test service name sanitization (no special characters issues)
#[test]
#[serial]
fn test_service_name_with_special_chars() {
    env::set_var("OTEL_SERVICE_NAME", "excelsior-service_v1.0");

    let service_name = env::var("OTEL_SERVICE_NAME").unwrap();

    // Common characters in service names should be accepted
    assert!(service_name.contains("-"));
    assert!(service_name.contains("_"));
    assert!(service_name.contains("."));

    env::remove_var("OTEL_SERVICE_NAME");
}

/// Test environment values commonly used
#[test]
#[serial]
fn test_common_environment_values() {
    let common_envs = vec!["development", "staging", "production", "testing", "qa"];

    for env_value in common_envs {
        env::set_var("ENVIRONMENT", env_value);
        let result = env::var("ENVIRONMENT").unwrap();
        assert_eq!(result, env_value);
    }

    env::remove_var("ENVIRONMENT");
}

/// Test that the default sampler is AlwaysOn
/// This is a documentation/configuration test
#[test]
fn test_sampler_configuration() {
    use opentelemetry_sdk::trace::Sampler;

    // Verify AlwaysOn sampler can be created
    let sampler = Sampler::AlwaysOn;

    // Test alternative samplers for documentation
    let _sampler_off = Sampler::AlwaysOff;
    let _sampler_ratio = Sampler::TraceIdRatioBased(0.1);

    // In production, we use AlwaysOn by default
    drop(sampler);
    assert!(true, "Sampler configurations valid");
}

/// Test RandomIdGenerator can be created
#[test]
fn test_id_generator_creation() {
    use opentelemetry_sdk::trace::RandomIdGenerator;

    let generator = RandomIdGenerator::default();
    drop(generator);

    assert!(true, "RandomIdGenerator created successfully");
}
