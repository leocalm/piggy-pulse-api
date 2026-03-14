use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::ResetPasswordRequest;
use crate::error::app_error::AppError;
use crate::service::auth::AuthService;

#[post("/reset-password", data = "<payload>")]
pub async fn reset_password(pool: &State<PgPool>, config: &State<Config>, payload: Json<ResetPasswordRequest>) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    auth.reset_password(&payload.token, &payload.password).await?;

    Ok(Status::Ok)
}
