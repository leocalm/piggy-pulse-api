use rocket::State;
use rocket::http::Status;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::ChangePasswordRequest;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::auth::AuthService;

#[put("/password", data = "<payload>")]
pub async fn change_password(
    pool: &State<PgPool>,
    config: &State<Config>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: CurrentUser,
    payload: Json<ChangePasswordRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    auth.change_password(
        &user.id,
        &payload.current_password,
        &payload.new_password,
        client_ip.0.clone(),
        user_agent.0.clone(),
    )
    .await?;

    Ok(Status::Ok)
}
