// The simple-requests pack's own integration tests, in their own file so the pack never
// edits the shared integration_test.rs (see "Writing a pack" in the README).
use axum::http::StatusCode;
use excelsior::routes;
use excelsior::state::build_app_state;
use tokio::net::TcpListener;

// Starts the whole app (every pack's state) on a free port.
async fn spawn_app() -> String {
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = routes::create_routes(build_app_state().await);
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });
    format!("http://127.0.0.1:{}", port)
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
