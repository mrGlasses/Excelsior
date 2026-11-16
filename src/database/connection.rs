use sqlx::{MySql, Pool, mysql::MySqlPoolOptions};
use std::time::Duration;
use tracing::info;

pub async fn init_db() -> Result<Pool<MySql>, sqlx::Error> {
    let database_builder = &format!(
        "mysql://{}:{}@{}:{}/{}",
        std::env::var("DATABASE_USER").expect("DATABASE_USER must be set."),
        std::env::var("DATABASE_PSWD").expect("DATABASE_PSWD must be set."),
        std::env::var("DATABASE_HOST").expect("DATABASE_HOST must be set."),
        std::env::var("DATABASE_PORT").expect("DATABASE_PORT must be set."),
        std::env::var("DATABASE_NAME").expect("DATABASE_NAME must be set."),
    );

    MySqlPoolOptions::new()
        .max_connections(5)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_builder)
        .await
}

// Applies every file in src/database/migrations that this database hasn't run yet.
// sqlx records each applied file (with a checksum) in the _sqlx_migrations table,
// so a restart applies nothing twice, and an applied file that was edited later
// makes this return an error instead of silently running different SQL.
pub async fn run_migrations(pool: &Pool<MySql>) -> Result<(), sqlx::migrate::MigrateError> {
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
// (a database that never ran them, before the first run).
async fn count_applied_migrations(pool: &Pool<MySql>) -> Result<i64, sqlx::Error> {
    let table_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables          WHERE table_schema = DATABASE() AND table_name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;
    if table_exists == 0 {
        return Ok(0);
    }
    sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}
