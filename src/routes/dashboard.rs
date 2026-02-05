use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::dashboard::{BudgetPerDayResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse, TotalAssetsResponse};
use crate::models::pagination::CursorParams;
use crate::models::transaction::TransactionResponse;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(clippy::result_large_err)]
fn parse_period_id(period_id: Option<String>) -> Result<Uuid, AppError> {
    let value = period_id.ok_or_else(|| AppError::BadRequest("Missing period_id query parameter".to_string()))?;
    Uuid::parse_str(&value).map_err(|e| AppError::uuid("Invalid budget period id", e))
}

/// Get balance per day for all accounts within a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/budget-per-day?<period_id>")]
pub async fn get_balance_per_day(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.balance_per_day(&budget_period_uuid, &current_user.id).await?))
}

/// Get spending breakdown per category for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/spent-per-category?<period_id>")]
pub async fn get_spent_per_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<SpentPerCategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.spent_per_category(&budget_period_uuid, &current_user.id).await?))
}

/// Get monthly burn-in statistics for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/monthly-burn-in?<period_id>")]
pub async fn get_monthly_burn_in(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<MonthlyBurnInResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.monthly_burn_in(&budget_period_uuid, &current_user.id).await?))
}

/// Get month progress for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/month-progress?<period_id>")]
pub async fn get_month_progress(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<MonthProgressResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.month_progress(&budget_period_uuid, &current_user.id).await?))
}

/// Get recent transactions for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/recent-transactions?<period_id>")]
pub async fn get_recent_transactions(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;
    let params = CursorParams { cursor: None, limit: Some(10) };
    let transactions = repo.get_transactions_for_period(&budget_period_uuid, &params, &current_user.id).await?;
    Ok(Json(transactions.iter().take(10).map(TransactionResponse::from).collect()))
}

/// Get total assets
#[openapi(tag = "Dashboard")]
#[get("/total-assets")]
pub async fn get_total_assets(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<TotalAssetsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.get_total_assets(&current_user.id).await?))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_balance_per_day,
        get_spent_per_category,
        get_monthly_burn_in,
        get_month_progress,
        get_recent_transactions,
        get_total_assets,
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_period_id;
    use crate::error::app_error::AppError;

    #[test]
    fn parse_period_id_missing_returns_bad_request() {
        let result = parse_period_id(None);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }
}
