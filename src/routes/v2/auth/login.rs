use rocket::State;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::post;
use rocket::serde::json::Json;
use rocket::time::Duration;
use sqlx::PgPool;
use validator::Validate;

use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{AuthenticatedResponse, LoginRequest, LoginResponse, TwoFactorChallengeResponse};
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::auth::{AuthService, V2LoginOutcome};

/// Set the session cookie after a successful login or register.
pub(crate) fn set_session_cookie(cookies: &CookieJar<'_>, config: &Config, session_id: uuid::Uuid, user_id: uuid::Uuid) {
    let ttl_seconds = config.session.ttl_seconds.max(60);
    let value = format!("{}:{}", session_id, user_id);
    cookies.add_private(
        Cookie::build(("user", value))
            .path("/")
            .secure(config.session.cookie_secure)
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(Duration::seconds(ttl_seconds))
            .build(),
    );
}

#[post("/login", data = "<payload>")]
pub async fn login(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    let ip = client_ip.0.as_deref().unwrap_or("unknown");
    let outcome = auth
        .login_v2(&payload.email, &payload.password, ip, client_ip.0.clone(), user_agent.0.clone())
        .await?;

    match outcome {
        V2LoginOutcome::Success { session_id, user } => {
            set_session_cookie(cookies, config, session_id, user.id);
            let user_response = auth.build_user_response(user).await?;
            Ok(Json(LoginResponse::Authenticated(AuthenticatedResponse::new(user_response, None))))
        }
        V2LoginOutcome::TwoFactorRequired { two_factor_token } => {
            Ok(Json(LoginResponse::TwoFactorChallenge(TwoFactorChallengeResponse::new(two_factor_token))))
        }
    }
}
