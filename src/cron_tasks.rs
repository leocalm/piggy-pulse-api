use crate::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::init_pool;

#[derive(Debug, Clone, Copy)]
pub struct GeneratePeriodsResult {
    pub users_processed: i64,
    pub periods_created: i64,
}

pub async fn cleanup_expired_tokens(config: &Config) -> Result<(), String> {
    let pool = init_pool(&config.database, config.logging.slow_query_ms)
        .await
        .map_err(|err| format!("Failed to initialize database pool: {err}"))?;

    let repo = PostgresRepository { pool: pool.clone() };

    if let Err(err) = repo.cleanup_expired().await {
        tracing::error!("Failed to clean up expired API tokens: {err:?}");
    }

    if let Err(err) = repo.cleanup_expired_pending_2fa_tokens().await {
        tracing::error!("Failed to clean up expired pending 2FA tokens: {err:?}");
    }

    pool.close().await;

    Ok(())
}

pub async fn generate_periods(config: &Config) -> Result<GeneratePeriodsResult, String> {
    let pool = init_pool(&config.database, config.logging.slow_query_ms)
        .await
        .map_err(|err| format!("Failed to initialize database pool: {err}"))?;

    let repo = PostgresRepository { pool: pool.clone() };
    let result = repo
        .generate_automatic_budget_periods()
        .await
        .map_err(|err| format!("Failed to generate automatic periods: {err:?}"))?;

    pool.close().await;

    Ok(GeneratePeriodsResult {
        users_processed: result.users_processed,
        periods_created: result.periods_created,
    })
}
