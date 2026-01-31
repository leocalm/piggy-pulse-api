use crate::config::DatabaseConfig;
use crate::error::app_error::AppError;
use deadpool_postgres::{Client, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use rocket::fairing::AdHoc;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::{Config, NoTls};

async fn init_pool(db_config: &DatabaseConfig) -> Pool {
    let mgr = Manager::from_config(
        Config::from_str(&db_config.url).expect("Error parsing DATABASE_URL"),
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );

    Pool::builder(mgr)
        .max_size(db_config.max_connections as usize)
        .wait_timeout(Some(Duration::from_secs(db_config.connection_timeout)))
        .create_timeout(Some(Duration::from_secs(db_config.connection_timeout)))
        .recycle_timeout(Some(Duration::from_secs(db_config.acquire_timeout)))
        .runtime(Runtime::Tokio1)
        .build()
        .expect("failed to build Postgres pool")
}

pub fn stage_db(db_config: DatabaseConfig) -> AdHoc {
    AdHoc::try_on_ignite("Postgres", |rocket| async move {
        let client = init_pool(&db_config).await;
        Ok(rocket.manage(client))
    })
}

pub async fn get_client(pool: &Pool) -> Result<Client, AppError> {
    pool.get().await.map_err(|e| AppError::pool("Failed to acquire database connection", e))
}
