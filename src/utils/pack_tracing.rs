// The pack registry for logging/tracing and for the last step of shutdown.
// Packs only ADD lines here (`merge=union` in .gitattributes). Write full paths.
use tracing_subscriber::{Layer, Registry};

// Extra tracing layers from packs (OpenTelemetry, for example), added next to the
// console logger in setup_tracing().
pub fn pack_tracing_layers() -> Vec<Box<dyn Layer<Registry> + Send + Sync>> {
    vec![
        crate::utils::otel_config::layer(),
        // One line per pack, e.g. `crate::utils::otel_config::layer(),`
    ]
}

// Runs once the server and the workers have stopped: packs flush what they still hold
// (buffered traces, for example).
pub fn pack_shutdown() {
    crate::utils::otel_config::shutdown_telemetry();
    // One line per pack, e.g. `crate::utils::otel_config::shutdown_telemetry();`
}
