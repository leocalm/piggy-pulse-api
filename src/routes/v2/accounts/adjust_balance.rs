use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AdjustBalanceRequest, EncryptedAccountResponse};
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[post("/<id>/adjust-balance", data = "<payload>")]
pub async fn adjust_balance(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    id: &str,
    payload: Json<AdjustBalanceRequest>,
) -> Result<Json<EncryptedAccountResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    Ok(Json(service.adjust_balance(&uuid, &payload, &user.id, &dek).await?))
}
