use crate::routes::create_routes;
use crate::utils::un_utils::start_message;
use dotenv::dotenv;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod routes;
mod utils;

#[tokio::main]
async fn main() {
    dotenv().ok();
    setup_tracing().await;

    let app = create_routes();

    let pre_port = std::env::var("MS_PORT").expect("MS_PORT must be set.");
    let port = pre_port.parse().expect("MS_PORT must be a number.");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    //info!("Excelsior listening on {}", addr); //tracing mode startup
    start_message(addr.to_string()).await; //default mode startup

    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(err) = graceful.await {
        error!("server error: {}", err);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
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
