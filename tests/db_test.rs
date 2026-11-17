// The MariaDB pack's own integration tests, in their own file so the pack never edits the
// shared integration_test.rs (see "Writing a pack" in the README). They need the test
// database from .env.test.
use axum::http::StatusCode;
use excelsior::database::connection::init_db;
use excelsior::routes;
use excelsior::state::build_app_state;
use sqlx::MySqlPool;
use tokio::net::TcpListener;

fn setup_test_env() {
    // In CI the workflow sets the variables; locally they come from .env.test.
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
}

// Starts the whole app (every pack's state, so the migrations run once) on a free port.
async fn spawn_app() -> String {
    setup_test_env();
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

async fn cleanup_test_data(pool: &MySqlPool) {
    sqlx::query("DELETE FROM t_Users WHERE name = 'Test User'")
        .execute(pool)
        .await
        .expect("Failed to clean up test data");
}

#[tokio::test]
async fn test_get_users() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/users", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
}

#[tokio::test]
async fn test_create_user() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/users", address))
        .json(&serde_json::json!({
            "name": "Test User"
        }))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(response.status().as_u16(), StatusCode::CREATED.as_u16());

    let pool = init_db().await.expect("Failed to connect to test database");
    cleanup_test_data(&pool).await;
}
