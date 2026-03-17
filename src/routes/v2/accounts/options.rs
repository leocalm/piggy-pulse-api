use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountOptionListResponse, AccountOptionResponse};
use crate::error::app_error::AppError;

#[get("/options")]
pub async fn get_account_options(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<AccountOptionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let options = repo.get_account_options_v2(&user.id).await?;

    let responses = options.into_iter().map(|(id, name, color)| AccountOptionResponse { id, name, color }).collect();

    Ok(Json(responses))
}
