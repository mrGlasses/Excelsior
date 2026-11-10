use crate::handlers::worker_handler::{heartbeat_loop, stuck_loop};
use crate::routes::create_routes;
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

    let app = create_routes();

    let pre_port = std::env::var("MS_PORT").expect("MS_PORT must be set.");
    let port = pre_port.parse().expect("MS_PORT must be a number.");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let server = TcpListener::bind(&addr).await.unwrap();

    // One token tells everyone to stop: cancelled by Ctrl+C/SIGTERM, or by a failing worker.
    let shutdown = CancellationToken::new();

    let shutdown_timeout =
        shutdown_timeout_from(std::env::var("WORKER_SHUTDOWN_TIMEOUT_SECS").ok());
    let mut workers = Workers::new(shutdown.clone(), shutdown_timeout);
    workers.spawn("heartbeat", heartbeat_loop);
    if std::env::var("DEMO_STUCK_WORKER").is_ok_and(|value| value == "true") {
        workers.spawn("stuck", stuck_loop);
    }

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
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
