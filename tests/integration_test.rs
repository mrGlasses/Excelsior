use axum::http::StatusCode;
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

#[tokio::test]
async fn test_protected_route_unauthorized() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/protected-enter", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::OK),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn test_protected_route_authorized() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/protected-enter", address))
        .header("X-Custom-Header", "secret-value")
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::UNAUTHORIZED),
        StatusCode::OK
    );
}

#[tokio::test]
async fn test_get_params() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/params/42/another_p/test-param", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    assert!(body.contains("42"));
    assert!(body.contains("test-param"));
}

#[tokio::test]
async fn test_get_question_with_filters() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/question_separator", address))
        .query(&[("name", "John"), ("age", "30"), ("active", "true")])
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    assert!(body.contains("John"));
    assert!(body.contains("30"));
    assert!(body.contains("true"));
}

#[tokio::test]
async fn test_post_body_data() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/body-data", address))
        .json(&serde_json::json!({
            "code": 2007,
            "message_text": "Test message"
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    assert!(body.contains("Test message"));
    assert!(body.contains("2007"));
}
