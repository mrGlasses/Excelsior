use crate::routes::create_routes;
use crate::state::build_app_state;
use crate::utils::pack_tracing::{pack_shutdown, pack_tracing_layers};
use crate::utils::un_utils::start_message;
use dotenv::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::warn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn service_starter() {
    dotenv().ok();
    setup_tracing().await;

    let app = create_routes(build_app_state().await);

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

    pack_shutdown();
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
