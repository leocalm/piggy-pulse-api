use crate::Config;
use crate::auth::CurrentUser;
use crate::database::pending_2fa_token::PendingTwoFaToken;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::ClientIp;
use crate::models::api_token::generate_token;
use crate::models::user::UserResponse;
use crate::service::auth::AuthService;
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, post};
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct TokenLoginRequest {
    pub email: String,
    pub password: String,
    pub device_name: String,
    pub device_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct Token2faRequest {
    pub two_factor_token: String,
    pub code: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TokenRefreshRequest {
    pub refresh_token: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct TokenRevokeRequest {
    pub refresh_token: String,
}

#[derive(Serialize, JsonSchema)]
pub struct TokenResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Serialize, JsonSchema)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

// ─── TTL helpers ─────────────────────────────────────────────────────────────

fn access_ttl(config: &Config) -> i64 {
    config.session.access_token_ttl_seconds.unwrap_or(3600)
}

fn refresh_ttl(config: &Config) -> i64 {
    config.session.refresh_token_ttl_seconds.unwrap_or(30 * 24 * 3600)
}

// ─── Private helper ──────────────────────────────────────────────────────────

async fn issue_tokens(
    repo: &PostgresRepository,
    config: &Config,
    user_id: &Uuid,
    device_name: &str,
    device_id: &str,
) -> Result<(String, String, i64), AppError> {
    let access_secs = access_ttl(config);
    let refresh_secs = refresh_ttl(config);

    let (access_plain, access_hash) = generate_token("pp_at_");
    let (refresh_plain, refresh_hash) = generate_token("pp_rt_");

    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(access_secs);
    let refresh_expires_at = now + chrono::Duration::seconds(refresh_secs);

    repo.create_api_token(
        user_id,
        access_hash,
        refresh_hash,
        device_name.to_string(),
        device_id,
        &expires_at,
        &refresh_expires_at,
    )
    .await?;

    Ok((access_plain, refresh_plain, access_secs))
}

// ─── Endpoints ───────────────────────────────────────────────────────────────

/// Authenticate with email + password and receive Bearer tokens
#[openapi(tag = "Token Auth")]
#[post("/token", data = "<payload>")]
pub async fn token_login(
    pool: &State<PgPool>,
    config: &State<Config>,
    client_ip: ClientIp,
    payload: Json<TokenLoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let ip = client_ip.0.as_deref().unwrap_or("unknown");
    let auth = AuthService::new(&repo, config);

    // Resolve user (dummy_verify if not found to prevent timing attacks)
    let user_opt = repo.get_user_by_email(&payload.email).await?;
    let user_id = user_opt.as_ref().map(|u| u.id);

    auth.check_login_rate_limit(user_id.as_ref(), ip, user_opt.as_ref().map(|u| (u.email.as_str(), u.name.as_str())))
        .await?;

    let user = match user_opt {
        Some(u) => u,
        None => {
            PostgresRepository::dummy_verify(&payload.password);
            return Err(AppError::InvalidCredentials);
        }
    };

    // Verify password
    if repo.verify_password(&user, &payload.password).await.is_err() {
        return Err(auth
            .handle_failed_password(&user.id, &user.email, &user.name, ip, client_ip.0.clone(), None)
            .await);
    }

    // Reset login rate limits on success
    let _ = repo.reset_login_rate_limit(&user.id, ip).await;

    // Check 2FA
    let two_factor = repo.get_two_factor_by_user(&user.id).await?;
    let has_2fa = two_factor.as_ref().map(|tf| tf.is_enabled).unwrap_or(false);

    if has_2fa {
        // Issue a short-lived pending token for the 2FA step
        let (two_fa_plain, two_fa_hash) = generate_token("pp_2fa_");
        let pending_expires_at = Utc::now() + chrono::Duration::seconds(300); // 5 minutes
        repo.create_pending_2fa_token(&user.id, &two_fa_hash, &payload.device_name, &payload.device_id, &pending_expires_at)
            .await?;
        return Err(AppError::TwoFactorTokenRequired {
            two_factor_token: two_fa_plain,
        });
    }

    // Issue tokens
    let (access_plain, refresh_plain, expires_in) = issue_tokens(&repo, config, &user.id, &payload.device_name, &payload.device_id).await?;

    Ok(Json(TokenResponse {
        user: UserResponse::from(&user),
        access_token: access_plain,
        refresh_token: refresh_plain,
        expires_in,
        token_type: "Bearer".to_string(),
    }))
}

/// Complete 2FA login with a pending 2FA token and TOTP/backup code
#[openapi(tag = "Token Auth")]
#[post("/token/2fa", data = "<payload>")]
pub async fn token_2fa_complete(pool: &State<PgPool>, config: &State<Config>, payload: Json<Token2faRequest>) -> Result<Json<TokenResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Hash the incoming two_factor_token to look up in DB
    let token_hash = hex::encode(Sha256::digest(payload.two_factor_token.as_bytes()));

    let pending: PendingTwoFaToken = repo.take_pending_2fa_token(&token_hash).await?.ok_or(AppError::Unauthorized)?;

    if pending.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    // Verify the TOTP/backup code
    let two_factor_data = repo.get_two_factor_by_user(&pending.user_id).await?.ok_or(AppError::Unauthorized)?;

    let auth = AuthService::new(&repo, config);
    auth.verify_two_factor(&pending.user_id, two_factor_data, &payload.code, None, None).await?;

    // Fetch the user
    let user = repo.get_user_by_id(&pending.user_id).await?.ok_or(AppError::Unauthorized)?;

    // Issue tokens
    let (access_plain, refresh_plain, expires_in) = issue_tokens(&repo, config, &user.id, &pending.device_name, &pending.device_id).await?;

    Ok(Json(TokenResponse {
        user: UserResponse::from(&user),
        access_token: access_plain,
        refresh_token: refresh_plain,
        expires_in,
        token_type: "Bearer".to_string(),
    }))
}

/// Rotate an access token using a refresh token
#[openapi(tag = "Token Auth")]
#[post("/token/refresh", data = "<payload>")]
pub async fn token_refresh(pool: &State<PgPool>, config: &State<Config>, payload: Json<TokenRefreshRequest>) -> Result<Json<RefreshResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let refresh_hash = hex::encode(Sha256::digest(payload.refresh_token.as_bytes()));

    let token = repo.find_by_refresh_hash(&refresh_hash).await?.ok_or(AppError::Unauthorized)?;

    if token.refresh_expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    let access_secs = access_ttl(config);
    let (access_plain, access_hash) = generate_token("pp_at_");
    let new_expires_at = Utc::now() + chrono::Duration::seconds(access_secs);

    repo.update_access_token(&token.id, access_hash, &new_expires_at).await?;

    Ok(Json(RefreshResponse {
        access_token: access_plain,
        expires_in: access_secs,
        token_type: "Bearer".to_string(),
    }))
}

/// Revoke a token (logout from a specific device) — requires Bearer auth
#[openapi(tag = "Token Auth")]
#[post("/token/revoke", data = "<payload>")]
pub async fn token_revoke(pool: &State<PgPool>, current_user: CurrentUser, payload: Json<TokenRevokeRequest>) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let refresh_hash = hex::encode(Sha256::digest(payload.refresh_token.as_bytes()));

    let token = repo
        .find_by_refresh_hash(&refresh_hash)
        .await?
        .ok_or(AppError::NotFound("Token not found".to_string()))?;

    if token.user_id != current_user.id {
        return Err(AppError::Forbidden);
    }

    repo.revoke(&token.id).await?;

    Ok(Status::NoContent)
}

// ─── Route registration ───────────────────────────────────────────────────────

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![token_login, token_2fa_complete, token_refresh, token_revoke]
}
