use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use serde::Deserialize;
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

/// Flexible 2FA verify request — accepts either a setup-only `{ code }` or
/// a login `{ twoFactorToken, code }` payload.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TwoFactorVerifyBody {
    two_factor_token: Option<String>,
    code: String,
}

/// Verify 2FA — handles both setup (authenticated, no twoFactorToken) and
/// login challenge (unauthenticated, requires twoFactorToken).
#[post("/verify", data = "<payload>")]
pub async fn verify_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &rocket::http::CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: Option<CurrentUser>,
    payload: Json<TwoFactorVerifyBody>,
) -> Result<Json<AuthenticatedResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let is_setup = user.is_some() && payload.two_factor_token.as_deref().map(|t| t.is_empty()).unwrap_or(true);

    if is_setup {
        // 2FA setup: authenticated user, no twoFactorToken
        let user = user.as_ref().unwrap();
        if payload.code.len() != 6 {
            return Err(AppError::BadRequest("code must be exactly 6 characters".into()));
        }
        let backup_codes = tfa.verify_setup(&user.id, &payload.code, client_ip.0.clone(), user_agent.0.clone()).await?;
        let auth = AuthService::new(&repo, config);
        let user_response = auth.get_user_response(&user.id).await?;
        Ok(Json(AuthenticatedResponse::with_backup_codes(user_response, backup_codes)))
    } else {
        // 2FA login: unauthenticated, requires valid twoFactorToken
        let token = payload.two_factor_token.as_deref().unwrap_or("");
        if token.is_empty() {
            return Err(AppError::Unauthorized);
        }
        let complete_req = TwoFactorCompleteRequest {
            two_factor_token: token.to_string(),
            code: payload.code.clone(),
        };
        complete_req.validate()?;

        let (logged_in_user, session_id) = tfa
            .verify_login(&complete_req.two_factor_token, &complete_req.code, client_ip.0.clone(), user_agent.0.clone())
            .await?;

        set_session_cookie(cookies, config, session_id, logged_in_user.id);

        let user_id = logged_in_user.id;
        drop(logged_in_user);

        let auth = AuthService::new(&repo, config);
        let (access_token, _) = auth.issue_bearer_token(&user_id).await?;
        let user_response = auth.get_user_response(&user_id).await?;

        let (stored_wrapped_dek, stored_dek_params) = repo.get_wrapped_dek(&user_id).await?;

        Ok(Json(AuthenticatedResponse::with_dek(
            user_response,
            Some(access_token),
            stored_wrapped_dek.map(|b| BASE64.encode(&b)),
            stored_dek_params,
        )))
    }
}
