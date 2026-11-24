use crate::routes::create_routes;
use crate::state::build_app_state;
use crate::utils::pack_tracing::{pack_shutdown, pack_tracing_layers};
use crate::utils::pack_workers::start_pack_workers;
use crate::utils::un_utils::start_message;
use crate::utils::workers::{Workers, shutdown_timeout_from};
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Returns true for a clean shutdown, false if a worker failed (main exits with code 1).
pub async fn service_starter() -> bool {
    dotenv().ok();
    setup_tracing().await;

    let clean_shutdown = run_service().await;

    // Here, not inside run_service(): every way out of it (a pack that couldn't start,
    // a failing worker, Ctrl+C) passes this line, so packs always get to flush (the
    // buffered traces, for example).
    pack_shutdown();

    clean_shutdown
}

async fn run_service() -> bool {
    // Each pack connects to its service here (see state.rs).
    let state = build_app_state().await;

    // One token tells everyone to stop: cancelled by Ctrl+C/SIGTERM, or by a failing worker.
    let shutdown = CancellationToken::new();

    let shutdown_timeout =
        shutdown_timeout_from(std::env::var("WORKER_SHUTDOWN_TIMEOUT_SECS").ok());
    let mut workers = Workers::new(shutdown.clone(), shutdown_timeout);
    // Before the server starts, so a pack with a missing setting stops the app right away
    // (the workers already spawned are stopped when `workers` is dropped).
    if !start_pack_workers(&mut workers, &state) {
        return false;
    }

    let app = create_routes(state);

    let pre_port = std::env::var("MS_PORT").expect("MS_PORT must be set.");
    let port = pre_port.parse().expect("MS_PORT must be a number.");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let server = TcpListener::bind(&addr).await.unwrap();

    //info!("Excelsior listening on {}", addr); //tracing mode startup
    start_message(addr.to_string()).await; //default mode startup

    let wait_for_signal = async {
        tokio::select! {
            _ = shutdown_signal() => shutdown.cancel(),
            // A worker failed and cancelled the token first: stop listening for signals.
            _ = shutdown.cancelled() => {},
        }
    };
    let http_server = async {
        axum::serve(server, app)
            .with_graceful_shutdown(shutdown.clone().cancelled_owned())
            .await
            .expect("server error");
    };

    // All three run together, and all three are dropped together if this function is
    // cancelled (as the integration tests do), so nothing keeps running in the background.
    let (_, _, any_worker_failed) =
        tokio::join!(wait_for_signal, http_server, workers.run_until_shutdown());
    info!("server stopped");

    !any_worker_failed
}
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    warn!("signal received, starting graceful shutdown");
}

pub async fn setup_tracing() {
    // try_init (not init) because in the test binary, several tests call this via
    // service_starter(); a global subscriber can only be set once per process, and any
    // call after the first is expected to no-op rather than panic.
    let _ = tracing_subscriber::registry()
        // Only when a pack gives layers: an empty list answers "not interested" to every
        // log call, which would switch off all of the app's own logs.
        .with(Some(pack_tracing_layers()).filter(|layers| !layers.is_empty()))
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}
