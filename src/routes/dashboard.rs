use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::dashboard::{
    BudgetPerDayResponse, BudgetStabilityResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryListResponse, TotalAssetsResponse,
};
use crate::models::pagination::CursorParams;
use crate::models::transaction::TransactionResponse;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(clippy::result_large_err)]
fn parse_period_id(period_id: Option<String>) -> Result<Uuid, AppError> {
    let value = period_id.ok_or_else(|| AppError::BadRequest("Missing period_id query parameter".to_string()))?;
    Uuid::parse_str(&value).map_err(|e| AppError::uuid("Invalid budget period id", e))
}

/// Get balance per day for all accounts within a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/budget-per-day?<period_id>")]
pub async fn get_balance_per_day(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<BudgetPerDayResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.balance_per_day(&budget_period_uuid, &current_user.id).await?))
}

/// Get spending breakdown per category for a budget period.
/// `percentage_spent` is returned in basis points (percent * 100). Example: 2534 = 25.34%.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/spent-per-category?<period_id>")]
pub async fn get_spent_per_category(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<SpentPerCategoryListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    let rows = repo.spent_per_category(&budget_period_uuid, &current_user.id).await?;
    Ok(Json(SpentPerCategoryListResponse(rows)))
}

/// Get monthly burn-in statistics for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/monthly-burn-in?<period_id>")]
pub async fn get_monthly_burn_in(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<MonthlyBurnInResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.monthly_burn_in(&budget_period_uuid, &current_user.id).await?))
}

/// Get month progress for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/month-progress?<period_id>")]
pub async fn get_month_progress(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<MonthProgressResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.month_progress(&budget_period_uuid, &current_user.id).await?))
}

/// Get recent transactions for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/recent-transactions?<period_id>")]
pub async fn get_recent_transactions(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<Vec<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    repo.get_budget_period(&budget_period_uuid, &current_user.id).await?;
    let params = CursorParams { cursor: None, limit: Some(10) };
    let transactions = repo.get_transactions_for_period(&budget_period_uuid, &params, &current_user.id).await?;
    Ok(Json(transactions.iter().take(10).map(TransactionResponse::from).collect()))
}

/// Get total assets
#[openapi(tag = "Dashboard")]
#[get("/total-assets")]
pub async fn get_total_assets(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<TotalAssetsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.get_total_assets(&current_user.id).await?))
}

/// Get budget stability for closed periods.
#[openapi(tag = "Dashboard")]
#[get("/budget-stability")]
pub async fn get_budget_stability(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<BudgetStabilityResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(repo.budget_stability(&current_user.id).await?))
}

/// Get net financial position for a budget period.
/// Returns 400 if `period_id` is missing ("Missing period_id query parameter") or invalid.
#[openapi(tag = "Dashboard")]
#[get("/net-position?<period_id>")]
pub async fn get_net_position(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
) -> Result<Json<NetPositionResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let budget_period_uuid = parse_period_id(period_id)?;
    Ok(Json(repo.get_net_position(&budget_period_uuid, &current_user.id).await?))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_balance_per_day,
        get_spent_per_category,
        get_monthly_burn_in,
        get_month_progress,
        get_recent_transactions,
        get_total_assets,
        get_budget_stability,
        get_net_position,
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

        (user_json["id"].as_str().expect("user id").to_string(), user_email)
    }

    async fn create_budget_period(client: &Client) -> String {
        let today = Utc::now().date_naive();
        let payload = serde_json::json!({
            "name": format!("Period {}", Uuid::new_v4()),
            "start_date": (today - Duration::days(10)).to_string(),
            "end_date": (today + Duration::days(10)).to_string(),
        });

        let response = client
            .post("/api/v1/budget_period/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        let body = response.into_string().await.expect("budget period response body");
        let json: Value = serde_json::from_str(&body).expect("valid budget period json");
        json["id"].as_str().expect("period id").to_string()
    }

    async fn create_account(client: &Client, account_type: &str, balance: i64) {
        let payload = serde_json::json!({
            "name": format!("{} {}", account_type, Uuid::new_v4()),
            "color": "#123456",
            "icon": "wallet",
            "account_type": account_type,
            "balance": balance,
            "spend_limit": null,
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn total_assets_returns_zero_without_accounts_or_transactions() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;

        let response = client.get("/api/v1/dashboard/total-assets").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("total assets response body");
        let json: Value = serde_json::from_str(&body).expect("valid total assets json");
        assert_eq!(json["total_assets"].as_i64().unwrap_or_default(), 0);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn total_assets_includes_account_balance_without_transactions() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        // create_currency(&client, "TST").await;

        let account_payload = serde_json::json!({
            "name": format!("Main {}", Uuid::new_v4()),
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Checking",
            "balance": 5000,
            "spend_limit": null
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(account_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let response = client.get("/api/v1/dashboard/total-assets").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("total assets response body");
        let json: Value = serde_json::from_str(&body).expect("valid total assets json");
        assert_eq!(json["total_assets"].as_i64().unwrap_or_default(), 5000);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn net_position_returns_empty_state_values_without_accounts() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        let period_id = create_budget_period(&client).await;

        let response = client.get(format!("/api/v1/dashboard/net-position?period_id={period_id}")).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("net position response body");
        let json: Value = serde_json::from_str(&body).expect("valid net position json");

        assert_eq!(json["account_count"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["total_net_position"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["change_this_period"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["liquid_balance"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["protected_balance"].as_i64().unwrap_or_default(), 0);
        assert_eq!(json["debt_balance"].as_i64().unwrap_or_default(), 0);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn net_position_returns_expected_breakdown_for_account_types() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        let period_id = create_budget_period(&client).await;

        create_account(&client, "Checking", 10_000).await;
        create_account(&client, "Wallet", 2_000).await;
        create_account(&client, "Allowance", -500).await;
        create_account(&client, "Savings", 30_000).await;
        create_account(&client, "CreditCard", -8_000).await;

        let response = client.get(format!("/api/v1/dashboard/net-position?period_id={period_id}")).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("net position response body");
        let json: Value = serde_json::from_str(&body).expect("valid net position json");

        assert_eq!(json["account_count"].as_i64().unwrap_or_default(), 5);
        assert_eq!(json["liquid_balance"].as_i64().unwrap_or_default(), 11_500);
        assert_eq!(json["protected_balance"].as_i64().unwrap_or_default(), 30_000);
        assert_eq!(json["debt_balance"].as_i64().unwrap_or_default(), -8_000);
        assert_eq!(json["total_net_position"].as_i64().unwrap_or_default(), 33_500);
        assert_eq!(json["change_this_period"].as_i64().unwrap_or_default(), 0);
    }
}
