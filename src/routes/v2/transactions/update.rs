use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::{TransactionResponse, UpdateTransactionRequest};
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[put("/<id>", data = "<payload>")]
pub async fn update_transaction(
    pool: &State<PgPool>,
    user: CurrentUser,
    id: &str,
    payload: Json<UpdateTransactionRequest>,
) -> Result<Json<TransactionResponse>, AppError> {
    let tx_id = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    let response = service.update_transaction(&tx_id, &payload, &user.id).await?;
    Ok(Json(response))
}
