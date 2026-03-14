use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::EmergencyDisableRequestBody;
use crate::error::app_error::AppError;
use crate::service::two_factor::TwoFactorService;

#[post("/emergency-disable/request", data = "<payload>")]
pub async fn emergency_disable_request(pool: &State<PgPool>, config: &State<Config>, payload: Json<EmergencyDisableRequestBody>) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    tfa.emergency_disable_request(&payload.email).await?;

    Ok(Status::Ok)
}
