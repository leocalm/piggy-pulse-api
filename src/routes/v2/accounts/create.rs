use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{CreateAccountRequest, EncryptedAccountResponse};
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[post("/", data = "<payload>")]
pub async fn create_account(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    payload: Json<CreateAccountRequest>,
) -> Result<(Status, Json<EncryptedAccountResponse>), AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    let response = service.create_account(&payload, &user.id, &dek).await?;
    Ok((Status::Created, Json(response)))
}
