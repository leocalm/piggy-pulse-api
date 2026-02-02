use crate::config::DatabaseConfig;
use rocket::fairing::AdHoc;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

async fn init_pool(db_config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .min_connections(db_config.min_connections)
        .acquire_timeout(Duration::from_secs(db_config.acquire_timeout))
        .idle_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&db_config.url)
        .await
}

pub fn stage_db(db_config: DatabaseConfig) -> AdHoc {
    AdHoc::try_on_ignite("Postgres (sqlx)", |rocket| async move {
        match init_pool(&db_config).await {
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
