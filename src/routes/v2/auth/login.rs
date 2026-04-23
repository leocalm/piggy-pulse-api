use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
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
            let user_id = user.id;
            set_session_cookie(cookies, config, session_id, user_id);
            drop(user);
            let (access_token, _) = auth.issue_bearer_token(&user_id).await?;
            let user_response = auth.get_user_response(&user_id).await?;

            // Fetch stored wrapped DEK to return to client
            let (stored_wrapped_dek, stored_dek_params) = repo.get_wrapped_dek(&user_id).await?;

            Ok(Json(LoginResponse::Authenticated(AuthenticatedResponse::with_dek(
                user_response,
                Some(access_token),
                stored_wrapped_dek.map(|b| BASE64.encode(&b)),
                stored_dek_params,
            ))))
        }
        V2LoginOutcome::TwoFactorRequired { two_factor_token } => {
            Ok(Json(LoginResponse::TwoFactorChallenge(TwoFactorChallengeResponse::new(two_factor_token))))
        }
    }
}
