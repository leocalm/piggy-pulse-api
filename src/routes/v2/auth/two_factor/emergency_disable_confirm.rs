use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::EmergencyDisableConfirmRequest;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::two_factor::TwoFactorService;

#[post("/emergency-disable/confirm", data = "<payload>")]
pub async fn emergency_disable_confirm(
    pool: &State<PgPool>,
    config: &State<Config>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<EmergencyDisableConfirmRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    tfa.emergency_disable_confirm(&payload.token, client_ip.0.clone(), user_agent.0.clone()).await?;

    Ok(Status::Ok)
}
