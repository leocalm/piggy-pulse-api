use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{AuthenticatedResponse, TwoFactorCompleteRequest, TwoFactorVerifyRequest};
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::routes::v2::auth::login::set_session_cookie;
use crate::service::auth::AuthService;
use crate::service::two_factor::TwoFactorService;

/// Verify 2FA setup (authenticated user confirming TOTP code during setup).
#[post("/verify", data = "<payload>", rank = 1)]
pub async fn verify_two_factor_setup(
    pool: &State<PgPool>,
    config: &State<Config>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: CurrentUser,
    payload: Json<TwoFactorVerifyRequest>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let _backup_codes = tfa.verify_setup(&user.id, &payload.code, client_ip.0.clone(), user_agent.0.clone()).await?;

    let auth = AuthService::new(&repo, config);
    let user_response = auth.get_user_response(&user.id).await?;
    // Note: backup codes are generated during verify_setup but not included in
    // AuthenticatedResponse. Users can retrieve them via POST /backup-codes/regenerate.
    Ok(Json(AuthenticatedResponse::new(user_response, None)))
}

/// Complete 2FA login challenge (unauthenticated, uses two_factor_token).
#[post("/verify", data = "<payload>", rank = 2)]
pub async fn verify_two_factor_login(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &rocket::http::CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<TwoFactorCompleteRequest>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let (user, session_id) = tfa
        .verify_login(&payload.two_factor_token, &payload.code, client_ip.0.clone(), user_agent.0.clone())
        .await?;

    set_session_cookie(cookies, config, session_id, user.id);

    let auth = AuthService::new(&repo, config);
    let user_response = auth.get_user_response(&user.id).await?;
    Ok(Json(AuthenticatedResponse::new(user_response, None)))
}
