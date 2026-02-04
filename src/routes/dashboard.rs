use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::dashboard::{BudgetPerDayResponse, DashboardResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};
use crate::models::transaction::TransactionResponse;
use crate::service::dashboard::DashboardService;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;

/// Get balance per day for all accounts
#[openapi(tag = "Dashboard")]
#[get("/budget-per-day")]
pub async fn get_balance_per_day(pool: &State<PgPool>, current_user: CurrentUser) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.balance_per_day(&current_user.id).await?))
}

/// Get spending breakdown per category
#[openapi(tag = "Dashboard")]
#[get("/spent-per-category")]
pub async fn get_spent_per_category(pool: &State<PgPool>, current_user: CurrentUser) -> Result<Json<Vec<SpentPerCategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.spent_per_category(&current_user.id).await?))
}

/// Get monthly burn-in statistics
#[openapi(tag = "Dashboard")]
#[get("/monthly-burn-in")]
pub async fn get_monthly_burn_in(pool: &State<PgPool>, current_user: CurrentUser) -> Result<Json<MonthlyBurnInResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.monthly_burn_in(&current_user.id).await?))
}

/// Get month progress for a budget period
#[openapi(tag = "Dashboard")]
#[get("/month-progress?<period_id>")]
pub async fn get_month_progress(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<MonthProgressResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let budget_period = repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;
    let dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.month_progress().await?))
}

/// Get recent transactions for a budget period
#[openapi(tag = "Dashboard")]
#[get("/recent-transactions?<period_id>")]
pub async fn get_recent_transactions(pool: &State<PgPool>, current_user: CurrentUser, period_id: String) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let budget_period = repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;
    let mut dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.recent_transactions(&current_user.id).await?))
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
