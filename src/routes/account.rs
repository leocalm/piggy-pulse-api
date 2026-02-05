use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::account::{AccountRequest, AccountResponse};
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

/// List all accounts with cursor-based pagination
#[openapi(tag = "Accounts")]
#[get("/?<cursor>&<limit>")]
pub async fn list_all_accounts(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<AccountResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let account_service = AccountService::new(&repo);
    let params = CursorParams::from_query(cursor, limit)?;

    let responses = account_service.list_accounts(&params, &current_user.id).await?;
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
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

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
            .post("/api/accounts/")
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

        let response = client.get("/api/accounts/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
