use sqlx::{Pool, Postgres, postgres::PgPoolOptions};
use std::time::Duration;
use tracing::info;

// Used when DATABASE_MAX_CONNECTIONS is not set.
pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;
// The pool keeps this many connections open even when idle (never more than the max).
const MIN_CONNECTIONS: u32 = 2;

pub async fn init_db() -> Result<Pool<Postgres>, sqlx::Error> {
    let database_builder = &format!(
        "postgres://{}:{}@{}:{}/{}",
        std::env::var("DATABASE_USER").expect("DATABASE_USER must be set."),
        std::env::var("DATABASE_PSWD").expect("DATABASE_PSWD must be set."),
        std::env::var("DATABASE_HOST").expect("DATABASE_HOST must be set."),
        std::env::var("DATABASE_PORT").expect("DATABASE_PORT must be set."),
        std::env::var("DATABASE_NAME").expect("DATABASE_NAME must be set."),
    );

    let max_connections = max_connections_from(std::env::var("DATABASE_MAX_CONNECTIONS").ok());

    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(MIN_CONNECTIONS.min(max_connections))
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_builder)
        .await
}

// Pool size from the DATABASE_MAX_CONNECTIONS value; unset means the default.
// Takes the value instead of reading the env var itself, so tests don't have to
// change the process environment.
pub fn max_connections_from(value: Option<String>) -> u32 {
    match value {
        None => DEFAULT_MAX_CONNECTIONS,
        Some(text) => match text.trim().parse::<u32>() {
            Ok(number) if number > 0 => number,
            _ => panic!("DATABASE_MAX_CONNECTIONS must be a number greater than 0."),
        },
    }
}

// Applies every file in src/database/migrations that this database hasn't run yet.
// sqlx records each applied file (with a checksum) in the _sqlx_migrations table,
// so a restart applies nothing twice, and an applied file that was edited later
// makes this return an error instead of silently running different SQL.
pub async fn run_migrations(pool: &Pool<Postgres>) -> Result<(), sqlx::migrate::MigrateError> {
    let applied_before = count_applied_migrations(pool).await?;

    sqlx::migrate!("./src/database/migrations")
        .run(pool)
        .await?;

    let applied_now = count_applied_migrations(pool).await? - applied_before;
    if applied_now == 0 {
        info!("migrations up to date");
    } else {
        info!("applied {} migrations", applied_now);
    }
    Ok(())
}

// How many migrations this database has recorded; 0 when the table doesn't exist yet
// (a brand-new database, before the first run).
async fn count_applied_migrations(pool: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
    let table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await?;
    if !table_exists {
        return Ok(0);
    }
    sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}
