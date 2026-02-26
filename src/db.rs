use crate::config::DatabaseConfig;
use log::LevelFilter;
use rocket::fairing::AdHoc;
use sqlx::ConnectOptions;
use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;

// Embed migrations into the binary at compile time.
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn init_pool(db_config: &DatabaseConfig, slow_query_ms: u64) -> Result<PgPool, sqlx::Error> {
    let connect_options = PgConnectOptions::from_str(&db_config.url)?
        .log_statements(LevelFilter::Debug)
        .log_slow_statements(LevelFilter::Warn, Duration::from_millis(slow_query_ms));

    let pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .min_connections(db_config.min_connections)
        .acquire_timeout(Duration::from_secs(db_config.acquire_timeout))
        .idle_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(connect_options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}

pub fn stage_db(db_config: DatabaseConfig, slow_query_ms: u64) -> AdHoc {
    AdHoc::try_on_ignite("Postgres (sqlx)", move |rocket| async move {
        match init_pool(&db_config, slow_query_ms).await {
            Ok(pool) => {
                tracing::info!("Database pool initialized successfully");
                Ok(rocket.manage(pool))
            }
            Err(e) => {
                tracing::error!("Failed to initialize database pool: {}", e);
                Err(rocket)
            }
        }
    })
}
