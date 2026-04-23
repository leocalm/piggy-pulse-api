use rocket::State;
use rocket::delete;
use rocket::http::Status;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[delete("/<id>")]
pub async fn delete_transaction(pool: &State<PgPool>, user: CurrentUser, dek: Dek, id: &str) -> Result<Status, AppError> {
    let tx_id = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid transaction id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    service.delete_transaction(&tx_id, &user.id, &dek).await?;
    Ok(Status::NoContent)
}
