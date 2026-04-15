use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::{EncryptedTransactionResponse, UpdateTransactionRequest};
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[put("/<id>", data = "<payload>")]
pub async fn update_transaction(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    id: &str,
    payload: Json<UpdateTransactionRequest>,
) -> Result<Json<EncryptedTransactionResponse>, AppError> {
    let tx_id = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    let response = service.update_transaction(&tx_id, &payload, &user.id, &dek).await?;
    Ok(Json(response))
}
