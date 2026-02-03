use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::transaction::TransactionRepository;
use crate::error::app_error::AppError;
use crate::error::json::JsonBody;
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::models::transaction::{TransactionRequest, TransactionResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<PgPool>,
    _current_user: CurrentUser,
    payload: JsonBody<TransactionRequest>,
) -> Result<(Status, Json<TransactionResponse>), AppError> {
    payload.validate()?;

    let start = std::time::Instant::now();

    let client_start = std::time::Instant::now();
    // No longer acquiring deadpool client; use PostgresRepository with PgPool
    let repo = PostgresRepository { pool: pool.inner().clone() };
    tracing::trace!("Using PgPool in {:?}", client_start.elapsed());

    let query_start = std::time::Instant::now();
    let tx = repo.create_transaction(&payload).await?;
    tracing::trace!("Created transaction in {:?}", query_start.elapsed());

    tracing::trace!("Total create_transaction time: {:?}", start.elapsed());
    Ok((Status::Created, Json(TransactionResponse::from(&tx))))
}

#[rocket::get("/?<period_id>&<cursor>&<limit>")]
pub async fn list_all_transactions(
    pool: &State<PgPool>,
    _current_user: CurrentUser,
    period_id: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<TransactionResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let txs = if let Some(period_id) = period_id {
        let uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
        repo.get_transactions_for_period(&uuid, &params).await?
    } else {
        repo.list_transactions(&params).await?
    };

    let responses: Vec<TransactionResponse> = txs.iter().map(TransactionResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

#[rocket::get("/<id>")]
pub async fn get_transaction(pool: &State<PgPool>, _current_user: CurrentUser, id: &str) -> Result<Json<TransactionResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    if let Some(tx) = repo.get_transaction_by_id(&uuid).await? {
        Ok(Json(TransactionResponse::from(&tx)))
    } else {
        Err(AppError::NotFound("Transaction not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_transaction(pool: &State<PgPool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    repo.delete_transaction(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_transaction(
    pool: &State<PgPool>,
    _current_user: CurrentUser,
    id: &str,
    payload: JsonBody<TransactionRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    let tx = repo.update_transaction(&uuid, &payload).await?;
    Ok(Json(TransactionResponse::from(&tx)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_transaction, list_all_transactions, get_transaction, delete_transaction, put_transaction]
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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/transactions/invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_transaction_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/transactions/bad-id").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_transactions_invalid_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/transactions/?period_id=invalid-uuid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
