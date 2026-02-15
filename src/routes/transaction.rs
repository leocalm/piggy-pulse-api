use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::error::json::JsonBody;
use crate::middleware::rate_limit::RateLimit;
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::models::transaction::{TransactionRequest, TransactionResponse};
use crate::models::transaction_summary::TransactionSummaryResponse;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new transaction
#[openapi(tag = "Transactions")]
#[post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: JsonBody<TransactionRequest>,
) -> Result<(Status, Json<TransactionResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tx = repo.create_transaction(&payload, &current_user.id).await?;

    Ok((Status::Created, Json(TransactionResponse::from(&tx))))
}

/// List all transactions with cursor-based pagination, optionally filtered by budget period
#[openapi(tag = "Transactions")]
#[get("/?<period_id>&<cursor>&<limit>")]
pub async fn list_all_transactions(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let txs = if let Some(period_id) = period_id {
        let uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
        repo.get_transactions_for_period(&uuid, &params, &current_user.id).await?
    } else {
        repo.list_transactions(&params, &current_user.id).await?
    };

    let responses: Vec<TransactionResponse> = txs.iter().map(TransactionResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get transaction summary for a period
#[openapi(tag = "Transactions")]
#[get("/summary?<period_id>")]
pub async fn transaction_summary(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: String,
) -> Result<Json<TransactionSummaryResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
    let summary = repo.get_transaction_summary(&uuid, &current_user.id).await?;
    Ok(Json(TransactionSummaryResponse::from(&summary)))
}

/// Get a transaction by ID
#[openapi(tag = "Transactions")]
#[get("/<id>")]
pub async fn get_transaction(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<TransactionResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    if let Some(tx) = repo.get_transaction_by_id(&uuid, &current_user.id).await? {
        Ok(Json(TransactionResponse::from(&tx)))
    } else {
        Err(AppError::NotFound("Transaction not found".to_string()))
    }
}

/// Delete a transaction by ID
#[openapi(tag = "Transactions")]
#[delete("/<id>")]
pub async fn delete_transaction(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    repo.delete_transaction(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a transaction by ID
#[openapi(tag = "Transactions")]
#[put("/<id>", data = "<payload>")]
pub async fn put_transaction(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: JsonBody<TransactionRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    let tx = repo.update_transaction(&uuid, &payload, &current_user.id).await?;
    Ok(Json(TransactionResponse::from(&tx)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        create_transaction,
        list_all_transactions,
        transaction_summary,
        get_transaction,
        delete_transaction,
        put_transaction
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_transaction_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/transactions/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_transaction_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/transactions/bad-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_transactions_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/transactions/?period_id=invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_transaction_summary_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/transactions/summary?period_id=invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
