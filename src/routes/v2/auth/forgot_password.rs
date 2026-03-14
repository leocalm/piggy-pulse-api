use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::ForgotPasswordRequest;
use crate::error::app_error::AppError;
use crate::service::auth::AuthService;

#[post("/forgot-password", data = "<payload>")]
pub async fn forgot_password(pool: &State<PgPool>, config: &State<Config>, payload: Json<ForgotPasswordRequest>) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    auth.forgot_password(&payload.email).await?;

    Ok(Status::Ok)
}
