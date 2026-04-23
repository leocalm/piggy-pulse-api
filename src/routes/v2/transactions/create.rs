use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::{CreateTransactionRequest, EncryptedTransactionResponse};
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<CreateTransactionRequest>,
) -> Result<(Status, Json<EncryptedTransactionResponse>), AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    let response = service.create_transaction(&payload, &user.id, &dek).await?;
    Ok((Status::Created, Json(response)))
}
