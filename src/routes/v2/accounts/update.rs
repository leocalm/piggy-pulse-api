use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountResponse, UpdateAccountRequest};
use crate::error::app_error::AppError;
use crate::models::account::AccountUpdateRequest;

#[put("/<id>", data = "<payload>")]
pub async fn update_account(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<UpdateAccountRequest>) -> Result<Json<AccountResponse>, AppError> {
    let fields = payload.fields();
    fields.validate()?;

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let v1_request = AccountUpdateRequest {
        name: fields.name.clone(),
        color: fields.color.clone(),
        icon: "wallet".to_string(),
        account_type: payload.model_account_type(),
        spend_limit: fields.spend_limit.map(|s| s as i32),
        next_transfer_amount: None,
    };

    let account = repo.update_account(&uuid, &v1_request, &user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}
