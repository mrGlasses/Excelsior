use anyhow::Result;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, SdkTracerProvider};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{Layer, Registry};

// Set by layer(), so shutdown_telemetry() can flush the spans still buffered at shutdown.
static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

pub fn init_telemetry() -> Result<SdkTracerProvider> {
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "excelsior".to_string());

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.clone()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new(
                "deployment.environment",
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string()),
            ),
        ])
        .build();

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&otlp_endpoint)
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build OTLP exporter - check OTEL_EXPORTER_OTLP_ENDPOINT");

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(resource)
        .with_sampler(Sampler::AlwaysOn) // For production, consider ParentBased(TraceIdRatioBased(0.1))
        .with_id_generator(RandomIdGenerator::default())
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    Ok(tracer_provider)
}

// Called from utils/pack_tracing.rs: the layer that sends every span to the OTLP collector.
// Built once per process (the tests start the app several times).
pub fn layer() -> Box<dyn Layer<Registry> + Send + Sync> {
    let tracer_provider = TRACER_PROVIDER.get_or_init(|| {
        init_telemetry()
            .expect("Failed to initialize OpenTelemetry - cannot proceed without telemetry")
    });
    OpenTelemetryLayer::new(tracer_provider.tracer("excelsior")).boxed()
}

// Called from utils/pack_tracing.rs (pack_shutdown) once the app has stopped: sends the
// spans still waiting in the batch (the exporter only sends every few seconds), then stops
// the exporter.
pub fn shutdown_telemetry() {
    let Some(provider) = TRACER_PROVIDER.get() else {
        return;
    };
    match provider.shutdown() {
        Ok(()) => info!("telemetry flushed"),
        // A second call: the first one already sent everything.
        Err(OTelSdkError::AlreadyShutdown) => {}
        Err(e) => warn!("telemetry flush failed, the last traces may be lost: {e}"),
    }
}
