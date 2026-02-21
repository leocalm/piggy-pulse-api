use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::account::{
    AccountBalanceHistoryPoint, AccountDetailResponse, AccountListResponse, AccountManagementResponse, AccountOptionResponse, AccountRequest, AccountResponse,
    AccountUpdateRequest, AccountsSummaryResponse, AdjustStartingBalanceRequest,
};
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::service::account::AccountService;
use rocket::serde::json::Json;
use rocket::{State, delete, get, http::Status, patch, post, put};
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

/// Get all accounts for the management view (includes archived, no period metrics)
#[openapi(tag = "Accounts")]
#[get("/management")]
pub async fn list_accounts_management(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<Vec<AccountManagementResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let accounts = repo.list_accounts_management(&current_user.id).await?;
    Ok(Json(accounts))
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

/// Delete an account by ID (only if no transactions exist)
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
    payload: Json<AccountUpdateRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let account = repo.update_account(&uuid, &payload, &current_user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}

/// Archive an account by ID
#[openapi(tag = "Accounts")]
#[patch("/<id>/archive")]
pub async fn archive_account(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    repo.archive_account(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Restore an archived account by ID
#[openapi(tag = "Accounts")]
#[patch("/<id>/restore")]
pub async fn restore_account(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    repo.restore_account(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Adjust the starting balance of an account (only if earliest budget period is open)
#[openapi(tag = "Accounts")]
#[post("/<id>/adjust-balance", data = "<payload>")]
pub async fn adjust_starting_balance(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<AdjustStartingBalanceRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let account = repo.adjust_starting_balance(&uuid, payload.new_balance, &current_user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}

/// Get accounts summary (Total Net Worth, Total Assets, Total Liabilities)
#[openapi(tag = "Accounts")]
#[get("/summary")]
pub async fn get_accounts_summary(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<AccountsSummaryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let (total_net_worth, total_assets, total_liabilities) = repo.get_accounts_summary(&current_user.id).await?;
    Ok(Json(AccountsSummaryResponse {
        total_net_worth,
        total_assets,
        total_liabilities,
    }))
}

/// Get account options for dropdowns (id, name, icon)
#[openapi(tag = "Accounts")]
#[get("/options")]
pub async fn get_account_options(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<Vec<AccountOptionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let options = repo.get_account_options(&current_user.id).await?;
    let responses = options.into_iter().map(|(id, name, icon)| AccountOptionResponse { id, name, icon }).collect();
    Ok(Json(responses))
}

/// Get account detail metrics for a budget period
#[openapi(tag = "Accounts")]
#[get("/<id>/detail?<period_id>")]
pub async fn get_account_detail(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    period_id: String,
) -> Result<Json<AccountDetailResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let account_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
    let detail = repo.get_account_detail(&account_uuid, &period_uuid, &current_user.id).await?;
    Ok(Json(detail))
}

/// Get balance history for an account
#[openapi(tag = "Accounts")]
#[get("/<id>/balance-history?<range>&<period_id>")]
pub async fn get_account_balance_history(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    range: String,
    period_id: Option<String>,
) -> Result<Json<Vec<AccountBalanceHistoryPoint>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let account_uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;

    let today = chrono::Utc::now().date_naive();
    let (start, end) = match range.as_str() {
        "30d" => (today - chrono::Duration::days(30), today),
        "90d" => (today - chrono::Duration::days(90), today),
        "1y" => (today - chrono::Duration::days(365), today),
        "period" => {
            let pid = period_id.ok_or_else(|| AppError::BadRequest("period_id required for range=period".to_string()))?;
            let period_uuid = Uuid::parse_str(&pid).map_err(|e| AppError::uuid("Invalid period id", e))?;
            let period = repo.get_budget_period(&period_uuid, &current_user.id).await?;
            (period.start_date, period.end_date)
        }
        _ => return Err(AppError::BadRequest("Invalid range. Use: period, 30d, 90d, 1y".to_string())),
    };

    let points = repo.get_account_balance_history(&account_uuid, start, end, &current_user.id).await?;
    Ok(Json(points))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_account,
        list_all_accounts,
        list_accounts_management,
        get_account,
        delete_account,
        put_account,
        archive_account,
        restore_account,
        adjust_starting_balance,
        get_accounts_summary,
        get_account_options,
        get_account_detail,
        get_account_balance_history
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use chrono::{Datelike, Duration, NaiveDate, Utc};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
    use uuid::Uuid;

    async fn create_user_and_auth(client: &Client) -> (String, String) {
        let unique = Uuid::new_v4();
        let payload = serde_json::json!({
            "name": format!("Test User {}", unique),
            "email": format!("test.user.{}@example.com", unique),
            "password": "CorrectHorseBatteryStaple!2026"
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

        // Set default currency to EUR for the user
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

        (user_id, user_email)
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

    async fn create_account(client: &Client, name: &str, balance: i64) -> String {
        let payload = serde_json::json!({
            "name": name,
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Checking",
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

    async fn create_transaction(client: &Client, category_id: &str, from_account_id: &str, occurred_at: NaiveDate, amount: i64) {
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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short
            "color": "#000000",
            "icon": "icon",
            "account_type": "Checking",
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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/accounts/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_accounts_includes_balance_metrics() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let category_id = create_category(&client, "Groceries", "Outgoing").await;
        let account_name = format!("Main {}", Uuid::new_v4());
        let account_id = create_account(&client, &account_name, 10_000).await;

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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

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
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let nonexistent_id = Uuid::new_v4();
        let response = client.get(format!("/api/v1/accounts/?period_id={}", nonexistent_id)).dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_accounts_summary() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        // Create asset accounts: Checking, Savings, Wallet
        create_account(&client, &format!("Checking {}", Uuid::new_v4()), 100_000).await;
        create_account(&client, &format!("Savings {}", Uuid::new_v4()), 50_000).await;

        // Create wallet account
        let wallet_payload = serde_json::json!({
            "name": format!("Wallet {}", Uuid::new_v4()),
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Wallet",
            "balance": 25_000,
            "spend_limit": null
        });
        let wallet_response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(wallet_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(wallet_response.status(), Status::Created);

        // Create credit card liability
        let credit_card_payload = serde_json::json!({
            "name": format!("Credit Card {}", Uuid::new_v4()),
            "color": "#654321",
            "icon": "card",
            "account_type": "CreditCard",
            "balance": 15_000,
            "spend_limit": null
        });
        let cc_response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(credit_card_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(cc_response.status(), Status::Created);

        // Get summary
        let response = client.get("/api/v1/accounts/summary").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("summary response body");
        let json: Value = serde_json::from_str(&body).expect("valid summary json");

        // Total assets: 100_000 + 50_000 + 25_000 = 175_000
        // Total liabilities: 15_000
        // Total net worth: 175_000 - 15_000 = 160_000
        assert_eq!(json["total_assets"].as_i64().unwrap_or_default(), 175_000);
        assert_eq!(json["total_liabilities"].as_i64().unwrap_or_default(), 15_000);
        assert_eq!(json["total_net_worth"].as_i64().unwrap_or_default(), 160_000);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_accounts_summary_empty() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/accounts/summary").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("summary response body");
        let json: Value = serde_json::from_str(&body).expect("valid summary json");

        assert_eq!(json["total_assets"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["total_liabilities"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["total_net_worth"].as_i64().unwrap_or_default(), 0);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_account_options_empty() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/accounts/options").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("options response body");
        let json: Value = serde_json::from_str(&body).expect("valid options json");

        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_account_options_multiple_accounts() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        // Create multiple accounts to test sorting
        create_account(&client, "Zebra Account", 100_000).await;
        create_account(&client, "Apple Account", 50_000).await;
        create_account(&client, "Banana Account", 25_000).await;

        let response = client.get("/api/v1/accounts/options").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("options response body");
        let json: Value = serde_json::from_str(&body).expect("valid options json");

        assert!(json.is_array());
        let accounts = json.as_array().unwrap();
        assert_eq!(accounts.len(), 3);

        // Verify sorting by name (alphabetically)
        assert_eq!(accounts[0]["name"].as_str().unwrap_or_default(), "Apple Account");
        assert_eq!(accounts[1]["name"].as_str().unwrap_or_default(), "Banana Account");
        assert_eq!(accounts[2]["name"].as_str().unwrap_or_default(), "Zebra Account");

        // Verify all required fields are present
        for account in accounts {
            assert!(account["id"].as_str().is_some());
            assert!(account["name"].as_str().is_some());
            assert!(account["icon"].as_str().is_some());
        }
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_archive_account() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let account_id = create_account(&client, &format!("Archive Me {}", Uuid::new_v4()), 5_000).await;

        let archive_response = client.patch(format!("/api/v1/accounts/{}/archive", account_id)).dispatch().await;
        assert_eq!(archive_response.status(), Status::Ok);

        // Management view should show account as archived
        let mgmt_response = client.get("/api/v1/accounts/management").dispatch().await;
        assert_eq!(mgmt_response.status(), Status::Ok);
        let body = mgmt_response.into_string().await.expect("management response body");
        let json: Value = serde_json::from_str(&body).expect("valid management json");
        let accounts = json.as_array().expect("accounts array");
        let archived = accounts
            .iter()
            .find(|a| a["id"].as_str().is_some_and(|id| id == account_id))
            .expect("account in management list");
        assert!(archived["is_archived"].as_bool().unwrap_or_default());
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_account_with_transactions_fails() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let category_id = create_category(&client, &format!("Cat {}", Uuid::new_v4()), "Outgoing").await;
        let account_id = create_account(&client, &format!("Has Transactions {}", Uuid::new_v4()), 10_000).await;
        let today = Utc::now().date_naive();
        create_transaction(&client, &category_id, &account_id, today, 100).await;

        let delete_response = client.delete(format!("/api/v1/accounts/{}", account_id)).dispatch().await;
        assert_eq!(delete_response.status(), Status::BadRequest);

        let body = delete_response.into_string().await.expect("error body");
        assert!(body.contains("Cannot delete account with existing transactions"));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_account_detail() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let category_id = create_category(&client, "Salary", "Incoming").await;
        let expense_id = create_category(&client, "Rent", "Outgoing").await;
        let account_id = create_account(&client, "Test Checking", 100_000).await;

        let today = Utc::now().date_naive();
        let start = today.checked_sub_signed(Duration::days(5)).unwrap_or(today);
        let period_id = create_budget_period(&client, start, today).await;

        create_transaction(&client, &category_id, &account_id, start, 50_000).await;
        create_transaction(&client, &expense_id, &account_id, start, 20_000).await;

        let response = client
            .get(format!("/api/v1/accounts/{}/detail?period_id={}", account_id, period_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("response body");
        let json: Value = serde_json::from_str(&body).expect("valid json");

        assert_eq!(json["inflows"].as_i64().unwrap(), 50_000);
        assert_eq!(json["outflows"].as_i64().unwrap(), 20_000);
        assert_eq!(json["net"].as_i64().unwrap(), 30_000);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_account_balance_history_period() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let category_id = create_category(&client, "Salary", "Incoming").await;
        let account_id = create_account(&client, "Test Checking", 100_000).await;
        let today = Utc::now().date_naive();
        let start = today.checked_sub_signed(Duration::days(5)).unwrap_or(today);
        let period_id = create_budget_period(&client, start, today).await;
        create_transaction(&client, &category_id, &account_id, start, 50_000).await;

        let response = client
            .get(format!("/api/v1/accounts/{}/balance-history?range=period&period_id={}", account_id, period_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let json: Value = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        assert!(!json.as_array().unwrap().is_empty());
        let point = &json[0];
        assert!(point["date"].as_str().is_some());
        assert!(point["balance"].as_i64().is_some());
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_account_balance_history_invalid_range() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        let account_id = create_account(&client, "Test Checking", 100_000).await;

        let response = client
            .get(format!("/api/v1/accounts/{}/balance-history?range=bad", account_id))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
    }
}
