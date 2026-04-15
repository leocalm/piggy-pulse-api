use rocket::State;
use rocket::delete;
use rocket::http::Status;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[delete("/<id>")]
pub async fn delete_account(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    service.delete_account(&uuid, &user.id).await?;
    Ok(Status::NoContent)
}
