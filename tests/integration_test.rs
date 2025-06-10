use dotenv::dotenv;
use std::net::TcpListener;

// Import the excelsior crate and its modules
use excelsior::routes;

// Helper function to set up the test environment
fn setup_test_env() {
    if std::path::Path::new(".env.test").exists() {
        dotenv::from_filename(".env.test").ok();
    } else {
        dotenv().ok();
        println!("Warning: .env.test not found, using .env file");
    }
}

// Helper function to create a test app instance
async fn spawn_app() -> String {
    setup_test_env();

    // Find a random available port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    // Build the application with routes
    let app = routes::create_routes();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn test_server_health_check() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/ping", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(response.text().await.unwrap(), "PONG!");
}
