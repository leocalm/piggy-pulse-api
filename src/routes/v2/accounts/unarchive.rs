use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountResponse;
use crate::error::app_error::AppError;

#[post("/<id>/unarchive")]
pub async fn unarchive_account(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<AccountResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    repo.restore_account(&uuid, &user.id).await?;

    let account = repo
        .get_account_by_id(&uuid, &user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

    Ok(Json(AccountResponse::from(&account)))
}
