// The PostgreSQL pack's own integration tests, in their own file so the pack never edits
// the shared integration_test.rs (see "Writing a pack" in the README). They need the test
// database from .env.test.
use axum::http::StatusCode;
use excelsior::database::connection::init_db;
use excelsior::routes;
use excelsior::state::build_app_state;
use sqlx::PgPool;
use tokio::net::TcpListener;

fn setup_test_env() {
    // In CI the workflow sets the variables; locally they come from .env.test.
    if std::env::var("CI").is_err() {
        dotenv::from_filename(".env.test").ok();
    }
}

// A connection for checking and cleaning up what the tests wrote.
async fn create_test_db_pool() -> PgPool {
    init_db().await.expect("Failed to connect to test database")
}

// Starts the whole app (every pack's state, which also runs the migrations) on a free port.
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

async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM t_users WHERE name = 'Test User'")
        .execute(pool)
        .await
        .expect("Failed to clean up test data");
}

#[tokio::test]
async fn test_get_users() {
    let address = spawn_app().await;
    let pool = create_test_db_pool().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/users", address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_create_user() {
    let address = spawn_app().await;
    let pool = create_test_db_pool().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/users", address))
        .json(&serde_json::json!({
            "name": "Test User"
        }))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::UNAUTHORIZED),
        StatusCode::CREATED
    );

    cleanup_test_data(&pool).await;
}

async fn count_users_named(pool: &PgPool, names: &[&str]) -> i64 {
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    sqlx::query_scalar("SELECT count(*) FROM t_users WHERE name = ANY($1)")
        .bind(names)
        .fetch_one(pool)
        .await
        .expect("Failed to count users")
}

async fn delete_users_named(pool: &PgPool, names: &[&str]) {
    let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    sqlx::query("DELETE FROM t_users WHERE name = ANY($1)")
        .bind(names)
        .execute(pool)
        .await
        .expect("Failed to clean up test users");
}

#[tokio::test]
async fn test_create_user_pair_commits_both() {
    let address = spawn_app().await;
    let pool = create_test_db_pool().await;
    let names = ["PairTestAnn", "PairTestBob"];
    delete_users_named(&pool, &names).await;

    let response = reqwest::Client::new()
        .post(format!("{}/users/pair", address))
        .json(&serde_json::json!({ "first": names[0], "second": names[1] }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), StatusCode::CREATED.as_u16());
    assert_eq!(count_users_named(&pool, &names).await, 2);

    delete_users_named(&pool, &names).await;
}

#[tokio::test]
async fn test_create_user_pair_rolls_back_when_second_fails() {
    let address = spawn_app().await;
    let pool = create_test_db_pool().await;
    let first = "PairTestCid";
    delete_users_named(&pool, &[first]).await;

    // t_users.name is VARCHAR(15), so the second insert fails after the first one ran.
    let response = reqwest::Client::new()
        .post(format!("{}/users/pair", address))
        .json(&serde_json::json!({ "first": first, "second": "NameLongerThan15" }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(
        response.status().as_u16(),
        StatusCode::INTERNAL_SERVER_ERROR.as_u16()
    );
    assert_eq!(count_users_named(&pool, &[first]).await, 0);
}

#[tokio::test]
async fn test_uuid_and_jsonb_round_trip() {
    setup_test_env();
    let pool = create_test_db_pool().await;
    // A temp table lives on one connection only, so everything runs in one
    // transaction; it's rolled back at the end and nothing is left behind.
    let mut tx = pool.begin().await.expect("Failed to begin transaction");

    sqlx::query(
        "CREATE TEMP TABLE uuid_json_check (id uuid PRIMARY KEY, data jsonb NOT NULL) ON COMMIT DROP",
    )
    .execute(&mut *tx)
    .await
    .expect("Failed to create temp table");

    let written_id: sqlx::types::Uuid = sqlx::query_scalar(
        "INSERT INTO uuid_json_check (id, data) VALUES (gen_random_uuid(), '{\"a\": 1}'::jsonb) RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .expect("Failed to insert uuid/jsonb row");

    let (read_id, read_data): (sqlx::types::Uuid, serde_json::Value) =
        sqlx::query_as("SELECT id, data FROM uuid_json_check")
            .fetch_one(&mut *tx)
            .await
            .expect("Failed to read uuid/jsonb row");

    assert_eq!(read_id, written_id);
    assert_eq!(read_data, serde_json::json!({ "a": 1 }));

    tx.rollback().await.expect("Failed to roll back");
}
