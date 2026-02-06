use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::dashboard::{BudgetPerDayResponse, MonthProgressResponse, MonthlyBurnInResponse, SpentPerCategoryListResponse, TotalAssetsResponse};
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

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_balance_per_day,
        get_spent_per_category,
        get_monthly_burn_in,
        get_month_progress,
        get_recent_transactions,
        get_total_assets,
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_period_id;
    use crate::error::app_error::AppError;
    use crate::routes::user::build_auth_cookie;
    use crate::{Config, build_rocket};
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
        client.cookies().add_private(build_auth_cookie(&cookie_value));

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

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn total_assets_returns_zero_without_accounts_or_transactions() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        create_user_and_auth(&client).await;
        create_currency(&client, "TST").await;

        let account_payload = serde_json::json!({
            "name": format!("Main {}", Uuid::new_v4()),
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Checking",
            "currency": "TST",
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
}
