use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::EncryptedAccountResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/<id>")]
pub async fn get_account(pool: &State<PgPool>, user: CurrentUser, id: &str) -> Result<Json<EncryptedAccountResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    Ok(Json(service.get_account(&uuid, &user.id).await?))
}
