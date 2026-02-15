use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::budget_period::{BudgetPeriodRequest, BudgetPeriodResponse};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new budget period
#[openapi(tag = "Budget Periods")]
#[post("/", data = "<payload>")]
pub async fn create_budget_period(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<BudgetPeriodRequest>,
) -> Result<(Status, String), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_id = repo.create_budget_period(&payload, &current_user.id).await?;
    Ok((Status::Created, budget_period_id.to_string()))
}

/// List all budget periods with cursor-based pagination
#[openapi(tag = "Budget Periods")]
#[get("/?<cursor>&<limit>")]
pub async fn list_budget_periods(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<BudgetPeriodResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let list = repo.list_budget_periods_with_metrics(&params, &current_user.id).await?;
    let responses: Vec<BudgetPeriodResponse> = list.iter().map(BudgetPeriodResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get the current budget period (whose date range covers today)
#[openapi(tag = "Budget Periods")]
#[get("/current")]
pub async fn get_current_budget_period(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<BudgetPeriodResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period = repo.get_current_budget_period_with_metrics(&current_user.id).await?;
    Ok(Json(BudgetPeriodResponse::from(&budget_period)))
}

/// Update a budget period by ID
#[openapi(tag = "Budget Periods")]
#[put("/<id>", data = "<payload>")]
pub async fn put_budget_period(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<BudgetPeriodRequest>,
) -> Result<Json<BudgetPeriodResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    let budget_period = repo.update_budget_period(&uuid, &payload, &current_user.id).await?;
    Ok(Json(BudgetPeriodResponse::from(&budget_period)))
}

/// Delete a budget period by ID
#[openapi(tag = "Budget Periods")]
#[delete("/<id>")]
pub async fn delete_budget_period(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;
    repo.delete_budget_period(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_budget_period,
        list_budget_periods,
        get_current_budget_period,
        put_budget_period,
        delete_budget_period,
        get_period_schedule,
        create_period_schedule,
        update_period_schedule,
        delete_period_schedule,
        get_period_gaps
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_put_budget_period_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.put("/api/v1/budget_period/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_budget_period_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/budget_period/not-valid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}

// ===== Period Schedule Routes =====

/// Get period schedule configuration
#[openapi(tag = "Budget Periods")]
#[get("/schedule")]
pub async fn get_period_schedule(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<crate::models::budget_period::PeriodScheduleResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let schedule = repo.get_period_schedule(&current_user.id).await?;
    Ok(Json(crate::models::budget_period::PeriodScheduleResponse::from(&schedule)))
}

/// Create period schedule configuration
#[openapi(tag = "Budget Periods")]
#[post("/schedule", data = "<payload>")]
pub async fn create_period_schedule(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<crate::models::budget_period::PeriodScheduleRequest>,
) -> Result<Json<crate::models::budget_period::PeriodScheduleResponse>, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let schedule = repo.create_period_schedule(&payload, &current_user.id).await?;
    Ok(Json(crate::models::budget_period::PeriodScheduleResponse::from(&schedule)))
}

/// Update period schedule configuration
#[openapi(tag = "Budget Periods")]
#[put("/schedule", data = "<payload>")]
pub async fn update_period_schedule(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<crate::models::budget_period::PeriodScheduleRequest>,
) -> Result<Json<crate::models::budget_period::PeriodScheduleResponse>, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let schedule = repo.update_period_schedule(&payload, &current_user.id).await?;
    Ok(Json(crate::models::budget_period::PeriodScheduleResponse::from(&schedule)))
}

/// Delete period schedule configuration
#[openapi(tag = "Budget Periods")]
#[delete("/schedule")]
pub async fn delete_period_schedule(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_period_schedule(&current_user.id).await?;
    Ok(Status::NoContent)
}

/// Get unassigned transactions (gaps)
#[openapi(tag = "Budget Periods")]
#[get("/gaps")]
pub async fn get_period_gaps(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<crate::models::budget_period::GapsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let gaps = repo.get_period_gaps(&current_user.id).await?;
    Ok(Json(gaps))
}
