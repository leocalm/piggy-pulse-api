use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::dashboard::{BudgetPerDayResponse, DashboardResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};
use crate::models::pagination::CursorParams;
use crate::models::transaction::TransactionResponse;
use crate::service::dashboard::DashboardService;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;

/// Get balance per day for all accounts within a budget period
#[openapi(tag = "Dashboard")]
#[get("/budget-per-day?<period_id>")]
pub async fn get_balance_per_day(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    Ok(Json(repo.balance_per_day(&budget_period_uuid, &current_user.id).await?))
}

/// Get spending breakdown per category for a budget period
#[openapi(tag = "Dashboard")]
#[get("/spent-per-category?<period_id>")]
pub async fn get_spent_per_category(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    period_id: String,
) -> Result<Json<Vec<SpentPerCategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    Ok(Json(repo.spent_per_category(&budget_period_uuid, &current_user.id).await?))
}

/// Get monthly burn-in statistics for a budget period
#[openapi(tag = "Dashboard")]
#[get("/monthly-burn-in?<period_id>")]
pub async fn get_monthly_burn_in(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<MonthlyBurnInResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    Ok(Json(repo.monthly_burn_in(&budget_period_uuid, &current_user.id).await?))
}

/// Get month progress for a budget period
#[openapi(tag = "Dashboard")]
#[get("/month-progress?<period_id>")]
pub async fn get_month_progress(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<MonthProgressResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    Ok(Json(repo.month_progress(&budget_period_uuid, &current_user.id).await?))
}

/// Get recent transactions for a budget period
#[openapi(tag = "Dashboard")]
#[get("/recent-transactions?<period_id>")]
pub async fn get_recent_transactions(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let params = CursorParams { cursor: None, limit: Some(10) };
    let transactions = repo.get_transactions_for_period(&budget_period_uuid, &params, &current_user.id).await?;
    Ok(Json(transactions.iter().take(10).map(TransactionResponse::from).collect()))
}

/// Get complete dashboard data for a budget period
#[openapi(tag = "Dashboard")]
#[get("/dashboard?<period_id>")]
pub async fn get_dashboard(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<DashboardResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let budget_period = repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;
    let mut dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.dashboard_response(&current_user.id).await?))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_balance_per_day,
        get_spent_per_category,
        get_monthly_burn_in,
        get_month_progress,
        get_recent_transactions,
        get_dashboard
    ]
}
