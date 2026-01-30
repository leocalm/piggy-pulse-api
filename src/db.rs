use crate::error::app_error::AppError;
use deadpool_postgres::{Client, Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use rocket::fairing::AdHoc;
use std::str::FromStr;
use tokio_postgres::{Config, NoTls};

async fn init_pool() -> Pool {
    let mgr = Manager::from_config(
        Config::from_str(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set")).expect("Error parsing DATABASE_URL"),
        NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );

    Pool::builder(mgr)
        .max_size(16)
        .wait_timeout(Some(std::time::Duration::from_secs(5)))
        .create_timeout(Some(std::time::Duration::from_secs(5)))
        .recycle_timeout(Some(std::time::Duration::from_secs(5)))
        .runtime(Runtime::Tokio1)
        .build()
        .expect("failed to build Postgres pool")
}

pub fn stage_db() -> AdHoc {
    AdHoc::try_on_ignite("Postgres", |rocket| async {
        let client = init_pool().await;
        Ok(rocket.manage(client))
    })
}

pub async fn get_client(pool: &Pool) -> Result<Client, AppError> {
    pool.get().await.map_err(|e| AppError::Db(e.to_string()))
}
