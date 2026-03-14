use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{AuthenticatedResponse, RegisterRequest};
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::routes::v2::auth::login::set_session_cookie;
use crate::service::auth::AuthService;

#[post("/register", data = "<payload>")]
pub async fn register(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &rocket::http::CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<RegisterRequest>,
) -> Result<(Status, Json<AuthenticatedResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    let (user, session_id) = auth
        .register(
            &payload.email,
            &payload.password,
            &payload.name,
            &payload.currency_id,
            user_agent.0.as_deref(),
            client_ip.0.as_deref(),
        )
        .await?;

    set_session_cookie(cookies, config, session_id, user.id);

    let user_response = auth.get_user_response(&user.id).await?;
    Ok((Status::Created, Json(AuthenticatedResponse::new(user_response, None))))
}
