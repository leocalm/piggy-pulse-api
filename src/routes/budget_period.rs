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

    let list = repo.list_budget_periods(&params, &current_user.id).await?;
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
    let budget_period = repo.get_current_budget_period(&current_user.id).await?;
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
        delete_budget_period
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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.put("/api/budget_period/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_budget_period_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/budget_period/not-valid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
