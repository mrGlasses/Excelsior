use crate::routes::create_routes;
use crate::utils::otel_config::{setup_tracing_with_otel, shutdown_telemetry};
use crate::utils::un_utils::start_message;
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{info, warn};

pub async fn service_starter() {
    dotenv().ok();
    setup_tracing_with_otel();

    let app = create_routes();

    let pre_port = std::env::var("MS_PORT").expect("MS_PORT must be set.");
    let port = pre_port.parse().expect("MS_PORT must be a number.");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let server = TcpListener::bind(&addr).await.unwrap();

    //info!("Excelsior listening on {}", addr); //tracing mode startup
    start_message(addr.to_string()).await; //default mode startup

    axum::serve(server, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    info!("Shutting down OpenTelemetry...");
    shutdown_telemetry();
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
