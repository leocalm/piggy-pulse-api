use rocket::State;
use rocket::delete;
use rocket::http::Status;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;

#[delete("/<id>")]
pub async fn delete_account(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Check the account exists first (V2 returns 404 for missing accounts)
    repo.get_account_by_id(&uuid, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

    repo.delete_account(&uuid, &user.id).await?;
    Ok(Status::NoContent)
}
