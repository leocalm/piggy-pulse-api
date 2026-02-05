use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::account::{AccountListResponse, AccountRequest, AccountResponse};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::service::account::AccountService;
use rocket::serde::json::Json;
use rocket::{State, delete, get, http::Status, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new account
#[openapi(tag = "Accounts")]
#[post("/", data = "<payload>")]
pub async fn create_account(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<AccountRequest>,
) -> Result<(Status, Json<AccountResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let account = repo.create_account(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(AccountResponse::from(&account))))
}

/// List all accounts with cursor-based pagination filtered by budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Accounts")]
#[get("/?<period_id>&<cursor>&<limit>")]
pub async fn list_all_accounts(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<AccountListResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let account_service = AccountService::new(&repo);
    let params = CursorParams::from_query(cursor, limit)?;

    // Parse and validate period_id
    let budget_period_id = period_id.ok_or_else(|| AppError::BadRequest("Missing period_id query parameter".to_string()))?;
    let budget_period_uuid = Uuid::parse_str(&budget_period_id).map_err(|e| AppError::uuid("Invalid budget period id", e))?;

    // Validate that the period exists
    repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;

    let responses = account_service.list_accounts(&params, &budget_period_uuid, &current_user.id).await?;
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get an account by ID
#[openapi(tag = "Accounts")]
#[get("/<id>")]
pub async fn get_account(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<AccountResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    if let Some(account) = repo.get_account_by_id(&uuid, &current_user.id).await? {
        Ok(Json(AccountResponse::from(&account)))
    } else {
        Err(AppError::NotFound("Account not found".to_string()))
    }
}

/// Delete an account by ID
#[openapi(tag = "Accounts")]
#[delete("/<id>")]
pub async fn delete_account(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    repo.delete_account(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update an account by ID
#[openapi(tag = "Accounts")]
#[put("/<id>", data = "<payload>")]
pub async fn put_account(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<AccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let account = repo.update_account(&uuid, &payload, &current_user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![create_account, list_all_accounts, get_account, delete_account, put_account]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use chrono::{Datelike, Duration, NaiveDate, Utc};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
    use uuid::Uuid;

    async fn create_user_and_auth(client: &Client) -> (String, String) {
        let unique = Uuid::new_v4();
        let payload = serde_json::json!({
            "name": format!("Test User {}", unique),
            "email": format!("test.user.{}@example.com", unique),
            "password": "password123"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id").to_string();
        let user_email = user_json["email"].as_str().expect("user email").to_string();

        let cookie_value = format!("{}:{}", user_id, user_email);
        client.cookies().add_private(Cookie::build(("user", cookie_value)).path("/").build());

        (user_id, user_email)
    }

    async fn create_currency(client: &Client, code: &str) {
        let payload = serde_json::json!({
            "name": format!("Test Currency {}", code),
            "symbol": "$",
            "currency": code,
            "decimal_places": 2
        });

        let response = client
            .post("/api/v1/currency/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
    }

    async fn create_category(client: &Client, name: &str, category_type: &str) -> String {
        let payload = serde_json::json!({
            "name": name,
            "color": "#123456",
            "icon": "cart",
            "parent_id": null,
            "category_type": category_type
        });

        let response = client
            .post("/api/v1/categories/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("category response body");
        let category_json: Value = serde_json::from_str(&body).expect("valid category json");
        category_json["id"].as_str().expect("category id").to_string()
    }

    async fn create_account(client: &Client, name: &str, currency: &str, balance: i64) -> String {
        let payload = serde_json::json!({
            "name": name,
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Checking",
            "currency": currency,
            "balance": balance,
            "spend_limit": null
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("account response body");
        let account_json: Value = serde_json::from_str(&body).expect("valid account json");
        account_json["id"].as_str().expect("account id").to_string()
    }

    async fn create_transaction(client: &Client, category_id: &str, from_account_id: &str, occurred_at: NaiveDate, amount: i32) {
        let payload = serde_json::json!({
            "amount": amount,
            "description": "Test transaction",
            "occurred_at": occurred_at.to_string(),
            "category_id": category_id,
            "from_account_id": from_account_id,
            "to_account_id": null,
            "vendor_id": null
        });

        let response = client
            .post("/api/v1/transactions/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
    }

    async fn create_budget_period(client: &Client, start_date: NaiveDate, end_date: NaiveDate) -> String {
        let payload = serde_json::json!({
            "name": format!("Period {}", Uuid::new_v4()),
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string()
        });

        let response = client
            .post("/api/v1/budget_period/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("period response body");
        let json: Value = serde_json::from_str(&body).expect("valid period json");
        json["id"].as_str().expect("period id").to_string()
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_account_validation_error() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short
            "color": "#000000",
            "icon": "icon",
            "account_type": "Checking",
            "currency": "USD",
            "balance": 0
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_account_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/accounts/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_accounts_includes_balance_metrics() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        create_currency(&client, "TST").await;

        let category_id = create_category(&client, "Groceries", "Outgoing").await;
        let account_name = format!("Main {}", Uuid::new_v4());
        let account_id = create_account(&client, &account_name, "TST", 10_000).await;

        let today = Utc::now().date_naive();
        let start_date = today
            .checked_sub_signed(Duration::days(2))
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("valid fallback date"));
        let period_id = create_budget_period(&client, start_date, today).await;
        create_transaction(&client, &category_id, &account_id, start_date, 2_500).await;

        let response = client.get(format!("/api/v1/accounts/?period_id={}", period_id)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("accounts response body");
        let json: Value = serde_json::from_str(&body).expect("valid accounts json");
        let data = json["data"].as_array().expect("accounts data array");
        let account_json = data
            .iter()
            .find(|item| item["id"].as_str().is_some_and(|id| id == account_id))
            .expect("account in response");

        assert_eq!(account_json["balance"].as_i64().unwrap_or_default(), 7_500);
        assert_eq!(account_json["balance_change_this_period"].as_i64().unwrap_or_default(), -2_500);
        assert_eq!(account_json["transaction_count"].as_i64().unwrap_or_default(), 1);

        let balance_per_day = account_json["balance_per_day"].as_array().expect("balance_per_day array");
        assert!(!balance_per_day.is_empty());

        let last_entry = balance_per_day.last().expect("last balance per day");
        assert_eq!(last_entry["account_name"].as_str().unwrap_or_default(), account_name);
        assert_eq!(last_entry["date"].as_str().unwrap_or_default(), today.to_string());
        assert_eq!(last_entry["balance"].as_i64().unwrap_or_default(), 7_500);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_accounts_missing_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/accounts/").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);

        let body = response.into_string().await.expect("error response body");
        assert!(body.contains("Missing period_id query parameter"));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_accounts_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/accounts/?period_id=invalid-uuid").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);

        let body = response.into_string().await.expect("error response body");
        assert!(body.contains("Invalid budget period id"));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_accounts_nonexistent_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let nonexistent_id = Uuid::new_v4();
        let response = client.get(format!("/api/v1/accounts/?period_id={}", nonexistent_id)).dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }
}
