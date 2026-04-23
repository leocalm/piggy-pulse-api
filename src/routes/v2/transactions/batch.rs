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

#[post("/batch", data = "<payload>")]
pub async fn batch_create_transactions(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<Vec<CreateTransactionRequest>>,
) -> Result<(Status, Json<Vec<EncryptedTransactionResponse>>), AppError> {
    if payload.is_empty() {
        return Ok((Status::Created, Json(vec![])));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);

    let results = service.batch_create_transactions(&payload, &user.id, &dek).await?;

    Ok((Status::Created, Json(results)))
}
