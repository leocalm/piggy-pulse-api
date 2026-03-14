use rocket::State;
use rocket::http::CookieJar;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::RefreshResponse;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::routes::v2::auth::login::set_session_cookie;
use crate::service::auth::AuthService;

#[post("/refresh")]
pub async fn refresh(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: CurrentUser,
) -> Result<Json<RefreshResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    let new_session_id = auth
        .refresh_session(&user.id, user.session_id, user_agent.0.as_deref(), client_ip.0.as_deref())
        .await?;

    // Re-stamp the session cookie with the new session
    set_session_cookie(cookies, config, new_session_id, user.id);

    Ok(Json(RefreshResponse {
        token: new_session_id.to_string(),
    }))
}
