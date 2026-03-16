use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountResponse, AdjustBalanceRequest};
use crate::error::app_error::AppError;

#[post("/<id>/adjust-balance", data = "<payload>")]
pub async fn adjust_balance(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<AdjustBalanceRequest>) -> Result<Json<AccountResponse>, AppError> {
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let account = repo.adjust_balance_v2(&uuid, payload.new_balance, &user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}
