use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::budget_category::{BudgetCategoryRequest, BudgetCategoryResponse};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new budget category
#[openapi(tag = "Budget Categories")]
#[post("/", data = "<payload>")]
pub async fn create_budget_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<BudgetCategoryRequest>,
) -> Result<(Status, Json<BudgetCategoryResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_category = repo.create_budget_category(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(BudgetCategoryResponse::from(&budget_category))))
}

/// List all budget categories with cursor-based pagination
#[openapi(tag = "Budget Categories")]
#[get("/?<cursor>&<limit>")]
pub async fn list_all_budget_categories(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<BudgetCategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let list = repo.list_budget_categories(&params, &current_user.id).await?;
    let responses: Vec<BudgetCategoryResponse> = list.iter().map(BudgetCategoryResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get a budget category by ID
#[openapi(tag = "Budget Categories")]
#[get("/<id>")]
pub async fn get_budget_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
) -> Result<Json<BudgetCategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget category id", e))?;
    if let Some(bc) = repo.get_budget_category_by_id(&uuid, &current_user.id).await? {
        Ok(Json(BudgetCategoryResponse::from(&bc)))
    } else {
        Err(AppError::NotFound("Budget category not found".to_string()))
    }
}

/// Delete a budget category by ID
#[openapi(tag = "Budget Categories")]
#[delete("/<id>")]
pub async fn delete_budget_category(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget category id", e))?;
    repo.delete_budget_category(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a budget category's budgeted value by ID
#[openapi(tag = "Budget Categories")]
#[put("/<id>", data = "<payload>")]
pub async fn put_budget_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<i32>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid budget category id", e))?;
    repo.update_budget_category_value(&uuid, &payload, &current_user.id).await?;
    Ok(Status::Ok)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_budget_category,
        list_all_budget_categories,
        get_budget_category,
        delete_budget_category,
        put_budget_category
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_budget_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/budget-categories/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_budget_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/budget-categories/bad-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
