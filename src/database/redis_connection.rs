use redis::aio::ConnectionManager;
use redis::{ConnectionAddr, IntoConnectionInfo, RedisConnectionInfo};

// Built up via ConnectionAddr/RedisConnectionInfo rather than a "redis://:{password}@..."
// URL string: the password isn't guaranteed to be URL-safe (e.g. a literal '#' truncates
// the URL at a "fragment", silently dropping everything after it, which fails to parse).
pub async fn init_cache() -> Result<ConnectionManager, redis::RedisError> {
    let redis_password = std::env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD must be set.");
    let redis_host = std::env::var("REDIS_HOST").expect("REDIS_HOST must be set.");
    let redis_port: u16 = std::env::var("REDIS_PORT")
        .expect("REDIS_PORT must be set.")
        .parse()
        .expect("REDIS_PORT must be a number.");

    println!(
        "Connecting to cache: redis://{}:{}/",
        redis_host, redis_port
    );

    let connection_info = ConnectionAddr::Tcp(redis_host, redis_port)
        .into_connection_info()?
        .set_redis_settings(RedisConnectionInfo::default().set_password(redis_password));

    let client = redis::Client::open(connection_info)?;
    ConnectionManager::new(client).await
}
