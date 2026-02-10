use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::category::{CategoryOption, CategoryRequest, CategoryResponse, CategoryWithStatsResponse};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new category
#[openapi(tag = "Categories")]
#[post("/", data = "<payload>")]
pub async fn create_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<CategoryRequest>,
) -> Result<(Status, Json<CategoryResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let category = repo.create_category(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(CategoryResponse::from(&category))))
}

/// List all categories with cursor-based pagination and stats for a selected budget period
#[openapi(tag = "Categories")]
#[get("/?<period_id>&<cursor>&<limit>")]
pub async fn list_all_categories(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: String,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<CategoryWithStatsResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;
    let period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
    let period = repo.get_budget_period(&period_uuid, &current_user.id).await?;

    let categories = repo.list_categories(&params, &current_user.id, &period).await?;
    let responses: Vec<CategoryWithStatsResponse> = categories.iter().map(CategoryWithStatsResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.category.id)))
}

/// Get category options (list of all categories for dropdowns/selection)
#[openapi(tag = "Categories")]
#[get("/options")]
pub async fn get_category_options(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<CategoryOption>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let categories = repo.list_all_categories(&current_user.id).await?;
    let options: Vec<CategoryOption> = categories.iter().map(CategoryOption::from).collect();
    Ok(Json(options))
}

/// Get a category by ID
#[openapi(tag = "Categories")]
#[get("/<id>")]
pub async fn get_category(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    if let Some(category) = repo.get_category_by_id(&uuid, &current_user.id).await? {
        Ok(Json(CategoryResponse::from(&category)))
    } else {
        Err(AppError::NotFound("Category not found".to_string()))
    }
}

/// Delete a category by ID
#[openapi(tag = "Categories")]
#[delete("/<id>")]
pub async fn delete_category(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    repo.delete_category(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a category by ID
#[openapi(tag = "Categories")]
#[put("/<id>", data = "<payload>")]
pub async fn put_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<CategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let category = repo.update_category(&uuid, &payload, &current_user.id).await?;
    Ok(Json(CategoryResponse::from(&category)))
}

/// List outgoing categories not yet associated with a budget
#[openapi(tag = "Categories")]
#[get("/not-in-budget?<cursor>&<limit>")]
pub async fn list_categories_not_in_budget(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<CategoryResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let categories = repo.list_categories_not_in_budget(&params, &current_user.id).await?;
    let responses: Vec<CategoryResponse> = categories.iter().map(CategoryResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_category,
        list_all_categories,
        get_category_options,
        get_category,
        delete_category,
        put_category,
        list_categories_not_in_budget
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use chrono::{Duration, Utc};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_category_validation_error() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short
            "color": "#000",
            "icon": "icon",
            "category_type": "Outgoing"
        });

        let response = client
            .post("/api/v1/categories/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/categories/invalid-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/categories/bad-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_categories_includes_stats() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let user_payload = serde_json::json!({
            "name": "Test User",
            "email": "test.user@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(user_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_email = user_json["email"].as_str().expect("user email");

        let login_payload = serde_json::json!({
            "email": user_email,
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let login_response = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(login_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(login_response.status(), Status::Ok);

        let currency_payload = serde_json::json!({
            "name": "US Dollar",
            "symbol": "$",
            "currency": "USD",
            "decimal_places": 2
        });

        let response = client
            .post("/api/v1/currency/")
            .header(ContentType::JSON)
            .body(currency_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let account_payload = serde_json::json!({
            "name": "Checking",
            "color": "#000000",
            "icon": "bank",
            "account_type": "Checking",
            "currency": "USD",
            "balance": 1000,
            "spend_limit": null
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(account_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("account response body");
        let account_json: Value = serde_json::from_str(&body).expect("valid account json");
        let account_id = account_json["id"].as_str().expect("account id");

        let category_payload = serde_json::json!({
            "name": "Groceries",
            "color": "#00FF00",
            "icon": "cart",
            "parent_id": null,
            "category_type": "Outgoing"
        });

        let response = client
            .post("/api/v1/categories/")
            .header(ContentType::JSON)
            .body(category_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("category response body");
        let category_json: Value = serde_json::from_str(&body).expect("valid category json");
        let category_id = category_json["id"].as_str().expect("category id");

        let today = Utc::now().date_naive();
        let period_payload = serde_json::json!({
            "name": "Test Period",
            "start_date": (today - Duration::days(1)).to_string(),
            "end_date": (today + Duration::days(1)).to_string()
        });

        let response = client
            .post("/api/v1/budget_period/")
            .header(ContentType::JSON)
            .body(period_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let period_id = response.into_string().await.expect("period id");

        let occurred_at = today.to_string();
        let tx_payload = serde_json::json!({
            "amount": 500,
            "description": "Groceries purchase",
            "occurred_at": occurred_at,
            "category_id": category_id,
            "from_account_id": account_id,
            "to_account_id": null,
            "vendor_id": null
        });

        let response = client
            .post("/api/v1/transactions/")
            .header(ContentType::JSON)
            .body(tx_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let response = client.get(format!("/api/v1/categories/?period_id={}&limit=50", period_id)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("categories response body");
        let list_json: Value = serde_json::from_str(&body).expect("valid categories json");
        let data = list_json["data"].as_array().expect("data array");
        let category = data.iter().find(|item| item["id"].as_str() == Some(category_id)).expect("category in list");

        assert_eq!(category["used_in_period"].as_i64(), Some(500));
        assert_eq!(category["transaction_count"].as_i64(), Some(1));
        assert_eq!(category["difference_vs_average_percentage"].as_i64(), Some(100));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_category_options() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/categories/options").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
    }
}
