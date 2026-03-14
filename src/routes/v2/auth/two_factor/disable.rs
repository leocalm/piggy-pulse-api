use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::TwoFactorDisableRequest;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::two_factor::TwoFactorService;

#[post("/disable", data = "<payload>")]
pub async fn disable_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: CurrentUser,
    payload: Json<TwoFactorDisableRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    tfa.disable(&user.id, &payload.code, client_ip.0.clone(), user_agent.0.clone()).await?;

    Ok(Status::Ok)
}
