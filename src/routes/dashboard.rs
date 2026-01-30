use crate::auth::CurrentUser;
use crate::database::budget_period::BudgetPeriodRepository;
use crate::database::dashboard::DashboardRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::dashboard::{BudgetPerDayResponse, DashboardResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryResponse};
use crate::models::transaction::TransactionResponse;
use crate::service::dashboard::DashboardService;
use deadpool_postgres::Pool;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;

#[rocket::get("/budget-per-day")]
pub async fn get_balance_per_day(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    Ok(Json(repo.balance_per_day().await?))
}

#[rocket::get("/spent-per-category")]
pub async fn get_spent_per_category(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<SpentPerCategoryResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    Ok(Json(repo.spent_per_category().await?))
}
#[rocket::get("/monthly-burn-in")]
pub async fn get_monthly_burn_in(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<MonthlyBurnInResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    Ok(Json(repo.monthly_burn_in().await?))
}

#[rocket::get("/month-progress?<period_id>")]
pub async fn get_month_progress(pool: &State<Pool>, _current_user: CurrentUser, period_id: String) -> Result<Json<MonthProgressResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period_uuid = Uuid::parse_str(&period_id)?;
    let budget_period = repo.get_budget_period(&budget_period_uuid).await?;
    let dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.month_progress().await?))
}

#[rocket::get("/recent-transactions?<period_id>")]
pub async fn get_recent_transactions(pool: &State<Pool>, _current_user: CurrentUser, period_id: String) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period_uuid = Uuid::parse_str(&period_id)?;
    let budget_period = repo.get_budget_period(&budget_period_uuid).await?;
    let mut dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.recent_transactions().await?))
}

#[rocket::get("/dashboard?<period_id>")]
pub async fn get_dashboard(pool: &State<Pool>, _current_user: CurrentUser, period_id: String) -> Result<Json<DashboardResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let budget_period_uuid = Uuid::parse_str(&period_id)?;
    let budget_period = repo.get_budget_period(&budget_period_uuid).await?;
    let mut dashboard_service = DashboardService::new(&repo, &budget_period);
    Ok(Json(dashboard_service.dashboard_response().await?))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        get_balance_per_day,
        get_spent_per_category,
        get_monthly_burn_in,
        get_month_progress,
        get_recent_transactions,
        get_dashboard
    ]
}
