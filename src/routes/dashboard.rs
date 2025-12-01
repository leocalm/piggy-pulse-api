use crate::auth::CurrentUser;
use crate::database::dashboard::{balance_per_day, monthly_burn_in, spent_per_category};
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::dashboard::{
    BudgetPerDayResponse, MonthlyBurnInResponse, SpentPerCategoryResponse,
};
use deadpool_postgres::Pool;
use rocket::serde::json::Json;
use rocket::State;

#[rocket::get("/dashboard/budget-per-day")]
pub async fn get_balance_per_day(
    pool: &State<Pool>,
    _current_user: CurrentUser,
) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let client = get_client(pool).await?;
    Ok(Json(balance_per_day(&client).await?))
}

#[rocket::get("/dashboard/spent-per-category")]
pub async fn get_spent_per_category(
    pool: &State<Pool>,
    _current_user: CurrentUser,
) -> Result<Json<Vec<SpentPerCategoryResponse>>, AppError> {
    let client = get_client(pool).await?;
    Ok(Json(spent_per_category(&client).await?))
}

#[rocket::get("/dashboard/monthly-burn-in")]
pub async fn get_monthly_burn_in(
    pool: &State<Pool>,
    _current_user: CurrentUser,
) -> Result<Json<Vec<MonthlyBurnInResponse>>, AppError> {
    let client = get_client(pool).await?;
    Ok(Json(monthly_burn_in(&client).await?))
}
