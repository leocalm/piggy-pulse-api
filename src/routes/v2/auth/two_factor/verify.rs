use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::{State, post};
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{AuthenticatedResponse, TwoFactorCompleteRequest};
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::routes::v2::auth::login::set_session_cookie;
use crate::service::auth::AuthService;
use crate::service::two_factor::TwoFactorService;

/// Confirm 2FA setup or complete a 2FA login challenge.
#[post("/verify", data = "<payload>")]
pub async fn verify_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: Option<CurrentUser>,
    payload: Json<TwoFactorCompleteRequest>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    if let Some(two_factor_token) = payload.two_factor_token.as_deref().map(str::trim).filter(|token| !token.is_empty()) {
        let (user, session_id) = tfa
            .verify_login(two_factor_token, &payload.code, client_ip.0.clone(), user_agent.0.clone())
            .await?;

        set_session_cookie(cookies, config, session_id, user.id);

        let user_id = user.id;
        drop(user);

        let auth = AuthService::new(&repo, config);
        let (access_token, _) = auth.issue_bearer_token(&user_id).await?;
        let user_response = auth.get_user_response(&user_id).await?;

        // Fetch stored wrapped DEK to return to client
        let (stored_wrapped_dek, stored_dek_params) = repo.get_wrapped_dek(&user_id).await?;

        return Ok(Json(AuthenticatedResponse::with_dek(
            user_response,
            Some(access_token),
            stored_wrapped_dek.map(|b| BASE64.encode(&b)),
            stored_dek_params,
        )));
    }

    let user = user.ok_or(AppError::Unauthorized)?;

    let backup_codes = tfa.verify_setup(&user.id, &payload.code, client_ip.0.clone(), user_agent.0.clone()).await?;

    let auth = AuthService::new(&repo, config);
    let user_response = auth.get_user_response(&user.id).await?;
    Ok(Json(AuthenticatedResponse::with_backup_codes(user_response, backup_codes)))
}
