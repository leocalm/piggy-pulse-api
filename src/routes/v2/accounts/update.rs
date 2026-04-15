use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{EncryptedAccountResponse, UpdateAccountRequest};
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[put("/<id>", data = "<payload>")]
pub async fn update_account(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    id: &str,
    payload: Json<UpdateAccountRequest>,
) -> Result<Json<EncryptedAccountResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    Ok(Json(service.update_account(&uuid, &payload, &user.id, &dek).await?))
}
