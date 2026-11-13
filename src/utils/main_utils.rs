use crate::engine::telegram_engine::{DEFAULT_TELEGRAM_API_URL, TelegramApi};
use crate::handlers::telegram_handler::{POLL_TIMEOUT_SECS, run_telegram_bot};
use crate::handlers::worker_handler::{heartbeat_loop, stuck_loop};
use crate::routes::create_routes;
use crate::utils::un_utils::start_message;
use crate::utils::workers::{Workers, shutdown_timeout_from};
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// Returns true for a clean shutdown, false if a worker failed (main exits with code 1).
pub async fn service_starter() -> bool {
    dotenv().ok();
    setup_tracing().await;

    // Checked first, so a missing token stops the app before anything else starts.
    let telegram_token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(token) if !token.trim().is_empty() => token,
        _ => {
            error!("TELEGRAM_BOT_TOKEN is missing");
            return false;
        }
    };
    let telegram_api_url =
        std::env::var("TELEGRAM_API_URL").unwrap_or_else(|_| DEFAULT_TELEGRAM_API_URL.to_string());
    let telegram_api = TelegramApi::new(&telegram_api_url, &telegram_token, POLL_TIMEOUT_SECS)
        .expect("Failed to build the Telegram HTTP client");

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
    workers.spawn("telegram", move |shutdown| {
        run_telegram_bot(Box::new(telegram_api), shutdown)
    });
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
    // try_init (not init) because in the test binary, several tests call this via
    // service_starter(); a global subscriber can only be set once per process, and any
    // call after the first is expected to no-op rather than panic.
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}
