use anyhow::Result;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use opentelemetry::metrics::Meter;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, SdkTracerProvider};
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{Layer, Registry};

// Set by layer(), so shutdown_telemetry() can flush the spans still buffered at shutdown.
static TRACER_PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();
// Set by init_telemetry(), so shutdown_telemetry() can flush the last metrics too.
static METER_PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();

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

    // Metrics go to the same collector, with the same service name.
    init_metrics(&otlp_endpoint, resource.clone());

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

// Counters and other metrics, exported over OTLP like the traces. The periodic reader
// sends every 60s; OTEL_METRIC_EXPORT_INTERVAL (in milliseconds) changes that.
fn init_metrics(otlp_endpoint: &str, resource: Resource) {
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(otlp_endpoint)
        .with_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build OTLP metric exporter - check OTEL_EXPORTER_OTLP_ENDPOINT");

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(resource)
        .build();

    global::set_meter_provider(meter_provider.clone());
    // Same rule as the tracer: the first one is kept, so shutdown_telemetry() can flush it.
    let _ = METER_PROVIDER.set(meter_provider);
}

#[cfg(test)]
pub(crate) fn is_meter_provider_set() -> bool {
    METER_PROVIDER.get().is_some()
}

// Demo counter: +1 per /ping. Grafana (Prometheus) shows it as excelsior_ping_total.
// Plugged in with one `.layer(...)` line in routes/mod.rs, so this pack never edits
// main's simple_handler.rs (see "Writing a pack" in the README).
pub async fn count_ping_requests(request: Request, next: Next) -> Response {
    if request.uri().path() == "/ping" {
        // Asked for on every call, not kept in a static: the global meter is read when
        // it's called, so one taken before the provider is set would stay a no-op forever.
        count_ping(&global::meter("excelsior"));
    }
    next.run(request).await
}

// Takes the meter as a parameter so a test can count with its own private provider.
pub fn count_ping(meter: &Meter) {
    meter
        .u64_counter("excelsior.ping")
        .with_description("Calls to /ping")
        .build()
        .add(1, &[]);
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

// Called from utils/pack_tracing.rs (pack_shutdown) once the app has stopped: sends what
// is still waiting (spans go out every few seconds, metrics every minute), then stops the
// exporters.
pub fn shutdown_telemetry() {
    let results = [
        ("traces", TRACER_PROVIDER.get().map(|p| p.shutdown())),
        ("metrics", METER_PROVIDER.get().map(|p| p.shutdown())),
    ];

    let mut flushed = false;
    let mut failed = false;
    for (what, result) in results {
        match result {
            Some(Ok(())) => flushed = true,
            // Never initialized, or a second call: the first one already sent everything.
            None | Some(Err(OTelSdkError::AlreadyShutdown)) => {}
            Some(Err(e)) => {
                failed = true;
                warn!("telemetry flush failed, the last {what} may be lost: {e}");
            }
        }
    }
    if flushed && !failed {
        info!("telemetry flushed");
    }
}
