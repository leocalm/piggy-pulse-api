use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::category::{
    CategoriesDiagnostics, CategoriesDiagnosticsResponse, CategoriesManagementListResponse, CategoryManagementResponse, CategoryOption, CategoryRequest,
    CategoryResponse, CategoryWithStatsResponse,
};
use crate::models::dashboard::PeriodContextSummaryResponse;
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

#[allow(clippy::result_large_err)]
fn parse_period_id(period_id: Option<String>) -> Result<Uuid, AppError> {
    let value = period_id.ok_or_else(|| AppError::BadRequest("Missing period_id query parameter".to_string()))?;
    Uuid::parse_str(&value).map_err(|e| AppError::uuid("Invalid period id", e))
}

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

/// Get categories diagnostics for the selected period.
#[openapi(tag = "Categories")]
#[get("/diagnostics?<period_id>")]
pub async fn get_categories_diagnostics(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<CategoriesDiagnosticsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let period_uuid = parse_period_id(period_id)?;
    let period = repo.get_budget_period(&period_uuid, &current_user.id).await?;

    let budgeted_rows = repo.list_budgeted_category_diagnostics(&current_user.id, &period).await?;
    let unbudgeted_rows = repo.list_unbudgeted_category_diagnostics(&current_user.id, &period).await?;
    let monthly_burn_in = repo.monthly_burn_in(&period.id, &current_user.id).await?;
    let month_progress = repo.month_progress(&period.id, &current_user.id).await?;

    Ok(Json(CategoriesDiagnosticsResponse::from(&CategoriesDiagnostics {
        period_summary: PeriodContextSummaryResponse::from_period_metrics(&monthly_burn_in, &month_progress),
        budgeted_rows,
        unbudgeted_rows,
    })))
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

/// List all categories for the management view (grouped by Incoming, Outgoing, Archived)
#[openapi(tag = "Categories")]
#[get("/management")]
pub async fn list_categories_for_management(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<CategoriesManagementListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let categories = repo.list_categories_for_management(&current_user.id).await?;

    let mut incoming: Vec<CategoryManagementResponse> = Vec::new();
    let mut outgoing: Vec<CategoryManagementResponse> = Vec::new();
    let mut archived: Vec<CategoryManagementResponse> = Vec::new();

    for row in categories {
        let response = CategoryManagementResponse::from(&row);
        if row.category.is_archived {
            archived.push(response);
        } else if row.category.category_type == crate::models::category::CategoryType::Incoming {
            incoming.push(response);
        } else {
            outgoing.push(response);
        }
    }

    Ok(Json(CategoriesManagementListResponse { incoming, outgoing, archived }))
}

/// Archive a category (soft delete)
#[openapi(tag = "Categories")]
#[post("/<id>/archive")]
pub async fn archive_category(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let category = repo.archive_category(&uuid, &current_user.id).await?;
    Ok(Json(CategoryResponse::from(&category)))
}

/// Restore an archived category
#[openapi(tag = "Categories")]
#[post("/<id>/restore")]
pub async fn restore_category(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<CategoryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid category id", e))?;
    let category = repo.restore_category(&uuid, &current_user.id).await?;
    Ok(Json(CategoryResponse::from(&category)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_category,
        list_all_categories,
        get_categories_diagnostics,
        get_category_options,
        get_category,
        delete_category,
        put_category,
        list_categories_not_in_budget,
        list_categories_for_management,
        archive_category,
        restore_category
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_period_id;
    use crate::error::app_error::AppError;
    use crate::{Config, build_rocket};
    use chrono::{Duration, Utc};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
    use uuid::Uuid;

    #[test]
    fn parse_period_id_missing_returns_bad_request() {
        let result = parse_period_id(None);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    async fn create_user_and_auth(client: &Client) {
        let unique = Uuid::new_v4();
        let user_payload = serde_json::json!({
            "name": format!("Test User {}", unique),
            "email": format!("test.user.{}@example.com", unique),
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

        let currency_response = client.get("/api/v1/currency/EUR").dispatch().await;
        assert_eq!(currency_response.status(), Status::Ok);
        let currency_body = currency_response.into_string().await.expect("currency response body");
        let currency_json: Value = serde_json::from_str(&currency_body).expect("valid currency json");
        let eur_id = currency_json["id"].as_str().expect("currency id");

        let settings_payload = serde_json::json!({
            "theme": "light",
            "language": "en",
            "default_currency_id": eur_id
        });
        let settings_response = client
            .put("/api/v1/settings")
            .header(ContentType::JSON)
            .body(settings_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(settings_response.status(), Status::Ok);
    }

    async fn create_account(client: &Client, name: &str) -> String {
        let account_payload = serde_json::json!({
            "name": name,
            "color": "#000000",
            "icon": "bank",
            "account_type": "Checking",
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
        let json: Value = serde_json::from_str(&body).expect("valid account json");
        json["id"].as_str().expect("account id").to_string()
    }

    async fn create_category(client: &Client, name: &str) -> String {
        let category_payload = serde_json::json!({
            "name": name,
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
        let json: Value = serde_json::from_str(&body).expect("valid category json");
        json["id"].as_str().expect("category id").to_string()
    }

    async fn create_budget_period(client: &Client, name: &str, start_date: String, end_date: String) -> String {
        let period_payload = serde_json::json!({
            "name": name,
            "start_date": start_date,
            "end_date": end_date
        });

        let response = client
            .post("/api/v1/budget_period/")
            .header(ContentType::JSON)
            .body(period_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
        response.into_string().await.expect("period id")
    }

    async fn create_budget_category(client: &Client, category_id: &str, budgeted_value: i32) {
        let payload = serde_json::json!({
            "category_id": category_id,
            "budgeted_value": budgeted_value
        });

        let response = client
            .post("/api/v1/budget-categories/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
    }

    async fn create_transaction(client: &Client, amount: i64, occurred_at: String, category_id: &str, account_id: &str) {
        let tx_payload = serde_json::json!({
            "amount": amount,
            "description": "Diagnostics transaction",
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
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_category_validation_error() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/categories/invalid-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_category_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/categories/bad-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_categories_includes_stats() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

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

        // Fetch EUR currency ID
        let currency_response = client.get("/api/v1/currency/EUR").dispatch().await;
        assert_eq!(currency_response.status(), Status::Ok);
        let currency_body = currency_response.into_string().await.expect("currency response body");
        let currency_json: Value = serde_json::from_str(&currency_body).expect("valid currency json");
        let eur_id = currency_json["id"].as_str().expect("currency id");

        // Set default currency
        let settings_payload = serde_json::json!({
            "theme": "light",
            "language": "en",
            "default_currency_id": eur_id
        });
        let settings_response = client
            .put("/api/v1/settings")
            .header(ContentType::JSON)
            .body(settings_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(settings_response.status(), Status::Ok);

        let account_payload = serde_json::json!({
            "name": "Checking",
            "color": "#000000",
            "icon": "bank",
            "account_type": "Checking",
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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/categories/options").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_categories_diagnostics_happy_path() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let account_id = create_account(&client, &format!("Checking {}", Uuid::new_v4())).await;

        let today = Utc::now().date_naive();
        let selected_period_id = create_budget_period(
            &client,
            &format!("Selected {}", Uuid::new_v4()),
            (today - Duration::days(1)).to_string(),
            (today + Duration::days(1)).to_string(),
        )
        .await;

        let _closed_period_id = create_budget_period(
            &client,
            &format!("Closed {}", Uuid::new_v4()),
            (today - Duration::days(8)).to_string(),
            (today - Duration::days(6)).to_string(),
        )
        .await;

        let budgeted_category_id = create_category(&client, &format!("Budgeted {}", Uuid::new_v4())).await;
        let unbudgeted_category_id = create_category(&client, &format!("Unbudgeted {}", Uuid::new_v4())).await;

        create_budget_category(&client, &budgeted_category_id, 1_000).await;
        create_transaction(&client, 700, today.to_string(), &budgeted_category_id, &account_id).await;
        create_transaction(&client, 300, today.to_string(), &unbudgeted_category_id, &account_id).await;
        create_transaction(&client, 900, (today - Duration::days(7)).to_string(), &budgeted_category_id, &account_id).await;

        let response = client
            .get(format!("/api/v1/categories/diagnostics?period_id={selected_period_id}"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("diagnostics response body");
        let json: Value = serde_json::from_str(&body).expect("valid diagnostics json");

        assert_eq!(json["period_summary"]["total_budget"].as_i64(), Some(1_000));
        assert_eq!(json["period_summary"]["spent_budget"].as_i64(), Some(1_000));
        assert_eq!(json["period_summary"]["remaining_budget"].as_i64(), Some(0));

        let budgeted_rows = json["budgeted_rows"].as_array().expect("budgeted rows");
        let budgeted = budgeted_rows
            .iter()
            .find(|item| item["id"].as_str() == Some(budgeted_category_id.as_str()))
            .expect("budgeted diagnostics row");
        assert_eq!(budgeted["budgeted_value"].as_i64(), Some(1_000));
        assert_eq!(budgeted["actual_value"].as_i64(), Some(700));
        assert_eq!(budgeted["variance_value"].as_i64(), Some(-300));
        assert_eq!(budgeted["progress_basis_points"].as_i64(), Some(7_000));
        assert_eq!(budgeted["recent_closed_periods"].as_array().map(Vec::len), Some(1));

        let unbudgeted_rows = json["unbudgeted_rows"].as_array().expect("unbudgeted rows");
        let unbudgeted = unbudgeted_rows
            .iter()
            .find(|item| item["id"].as_str() == Some(unbudgeted_category_id.as_str()))
            .expect("unbudgeted diagnostics row");
        assert_eq!(unbudgeted["actual_value"].as_i64(), Some(300));
        assert_eq!(unbudgeted["share_of_total_basis_points"].as_i64(), Some(10_000));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_categories_diagnostics_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/categories/diagnostics?period_id=invalid-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_categories_diagnostics_missing_period_id() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/categories/diagnostics").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_categories_diagnostics_stability_less_than_three_periods() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let account_id = create_account(&client, &format!("Checking {}", Uuid::new_v4())).await;
        let today = Utc::now().date_naive();

        let selected_period_id = create_budget_period(
            &client,
            &format!("Selected {}", Uuid::new_v4()),
            (today - Duration::days(1)).to_string(),
            (today + Duration::days(1)).to_string(),
        )
        .await;
        let _closed_period_id = create_budget_period(
            &client,
            &format!("Closed {}", Uuid::new_v4()),
            (today - Duration::days(8)).to_string(),
            (today - Duration::days(6)).to_string(),
        )
        .await;

        let budgeted_category_id = create_category(&client, &format!("Budgeted {}", Uuid::new_v4())).await;
        create_budget_category(&client, &budgeted_category_id, 1_000).await;
        create_transaction(&client, 950, (today - Duration::days(7)).to_string(), &budgeted_category_id, &account_id).await;

        let response = client
            .get(format!("/api/v1/categories/diagnostics?period_id={selected_period_id}"))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("diagnostics response body");
        let json: Value = serde_json::from_str(&body).expect("valid diagnostics json");
        let budgeted_rows = json["budgeted_rows"].as_array().expect("budgeted rows");
        let budgeted = budgeted_rows
            .iter()
            .find(|item| item["id"].as_str() == Some(budgeted_category_id.as_str()))
            .expect("budgeted diagnostics row");
        let periods = budgeted["recent_closed_periods"].as_array().expect("stability periods");
        assert_eq!(periods.len(), 1);
    }
}
