use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::two_factor::{
    EmergencyDisableConfirm, EmergencyDisableRequest, TwoFactorDisableRequest, TwoFactorRegenerateRequest, TwoFactorSetupResponse, TwoFactorStatus,
    TwoFactorVerifyRequest,
};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post};
use rocket_okapi::openapi;
use sqlx::PgPool;

/// Initialize 2FA setup - generates secret, QR code, and backup codes
#[openapi(tag = "Two-Factor Authentication")]
#[post("/setup")]
pub async fn setup_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
) -> Result<Json<TwoFactorSetupResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Check if user already has 2FA enabled
    if let Some(existing) = repo.get_two_factor_by_user(&current_user.id).await?
        && existing.is_enabled
    {
        return Err(AppError::BadRequest(
            "Two-factor authentication is already enabled. Disable it first to set up again.".to_string(),
        ));
    }

    // Parse encryption key from config
    let encryption_key = config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

    // Generate crypto material in blocking task (RNG operations)
    let issuer_name = config.two_factor.issuer_name.clone();
    let username = current_user.username.clone();
    let (secret, encrypted_secret, nonce, qr_code) = tokio::task::spawn_blocking(move || {
        // Generate TOTP secret
        let secret = PostgresRepository::generate_totp_secret();

        // Encrypt the secret before storing
        let (encrypted_secret, nonce) = PostgresRepository::encrypt_secret(&secret, &encryption_key)?;

        // Generate QR code
        let qr_code = PostgresRepository::generate_qr_code(&secret, &issuer_name, &username)?;

        Ok::<_, AppError>((secret, encrypted_secret, nonce, qr_code))
    })
    .await
    .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

    // Store encrypted secret (not yet enabled)
    repo.create_two_factor_setup(&current_user.id, &encrypted_secret, &nonce).await?;

    // Generate backup codes
    let backup_codes = repo.generate_backup_codes(&current_user.id).await?;

    Ok(Json(TwoFactorSetupResponse { secret, qr_code, backup_codes }))
}

/// Verify 2FA code and enable two-factor authentication
#[openapi(tag = "Two-Factor Authentication")]
#[post("/verify", data = "<payload>")]
pub async fn verify_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<TwoFactorVerifyRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Get the user's 2FA configuration
    let two_factor = repo
        .get_two_factor_by_user(&current_user.id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Two-factor authentication setup not found. Please initialize setup first.".to_string()))?;

    if two_factor.is_enabled {
        return Err(AppError::BadRequest("Two-factor authentication is already enabled.".to_string()));
    }

    // Parse encryption key
    let encryption_key = config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

    // Decrypt the secret
    let secret = PostgresRepository::decrypt_secret(&two_factor.encrypted_secret, &two_factor.encryption_nonce, &encryption_key)?;

    // Verify the TOTP code
    let is_valid = PostgresRepository::verify_totp_code(&secret, &payload.code)?;

    if !is_valid {
        return Err(AppError::BadRequest("Invalid verification code.".to_string()));
    }

    // Enable 2FA
    repo.verify_and_enable_two_factor(&current_user.id).await?;

    Ok(Status::Ok)
}

/// Disable two-factor authentication (requires password + current 2FA code)
#[openapi(tag = "Two-Factor Authentication")]
#[delete("/disable", data = "<payload>")]
pub async fn disable_two_factor(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<TwoFactorDisableRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Get the user to verify password
    let user = repo.get_user_by_id(&current_user.id).await?.ok_or(AppError::UserNotFound)?;

    // Verify password
    repo.verify_password(&user, &payload.password).await?;

    // Get the user's 2FA configuration
    let two_factor = repo
        .get_two_factor_by_user(&current_user.id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Two-factor authentication is not enabled.".to_string()))?;

    if !two_factor.is_enabled {
        return Err(AppError::BadRequest("Two-factor authentication is not enabled.".to_string()));
    }

    // Parse encryption key
    let encryption_key = config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

    // Decrypt the secret
    let secret = PostgresRepository::decrypt_secret(&two_factor.encrypted_secret, &two_factor.encryption_nonce, &encryption_key)?;

    // Verify the 2FA code OR backup code
    let totp_valid = PostgresRepository::verify_totp_code(&secret, &payload.code)?;
    let backup_valid = if !totp_valid {
        repo.verify_backup_code(&current_user.id, &payload.code).await?
    } else {
        false
    };

    if !totp_valid && !backup_valid {
        return Err(AppError::BadRequest("Invalid two-factor code.".to_string()));
    }

    // Disable 2FA (deletes all 2FA data)
    repo.disable_two_factor(&current_user.id).await?;

    Ok(Status::Ok)
}

/// Get current 2FA status for the authenticated user
#[openapi(tag = "Two-Factor Authentication")]
#[get("/status")]
pub async fn get_two_factor_status(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<TwoFactorStatus>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let two_factor = repo.get_two_factor_by_user(&current_user.id).await?;

    let enabled = two_factor.map(|tf| tf.is_enabled).unwrap_or(false);

    let (has_backup_codes, backup_codes_remaining) = if enabled {
        let count = repo.count_unused_backup_codes(&current_user.id).await?;
        (count > 0, count)
    } else {
        (false, 0)
    };

    Ok(Json(TwoFactorStatus {
        enabled,
        has_backup_codes,
        backup_codes_remaining,
    }))
}

/// Regenerate backup codes (requires current 2FA code)
#[openapi(tag = "Two-Factor Authentication")]
#[post("/regenerate-backup-codes", data = "<payload>")]
pub async fn regenerate_backup_codes(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<TwoFactorRegenerateRequest>,
) -> Result<Json<Vec<String>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Get the user's 2FA configuration
    let two_factor = repo
        .get_two_factor_by_user(&current_user.id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Two-factor authentication is not enabled.".to_string()))?;

    if !two_factor.is_enabled {
        return Err(AppError::BadRequest("Two-factor authentication is not enabled.".to_string()));
    }

    // Parse encryption key
    let encryption_key = config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

    // Decrypt and verify in blocking task
    let encrypted_secret = two_factor.encrypted_secret.clone();
    let encryption_nonce = two_factor.encryption_nonce.clone();
    let code = payload.code.clone();
    tokio::task::spawn_blocking(move || {
        // Decrypt the secret
        let secret = PostgresRepository::decrypt_secret(&encrypted_secret, &encryption_nonce, &encryption_key)?;

        // Verify the 2FA code
        let is_valid = PostgresRepository::verify_totp_code(&secret, &code)?;

        if !is_valid {
            return Err(AppError::BadRequest("Invalid two-factor code.".to_string()));
        }

        Ok::<_, AppError>(())
    })
    .await
    .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

    // Generate new backup codes
    let backup_codes = repo.generate_backup_codes(&current_user.id).await?;

    Ok(Json(backup_codes))
}

/// Request emergency 2FA disable via email
#[openapi(tag = "Two-Factor Authentication")]
#[post("/emergency-disable-request", data = "<payload>")]
pub async fn emergency_disable_request(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: RateLimit,
    payload: Json<EmergencyDisableRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Get user by email
    let user = match repo.get_user_by_email(&payload.email).await? {
        Some(u) => u,
        None => {
            // Don't reveal whether the account exists
            // Return success but don't send email
            return Ok(Status::Ok);
        }
    };

    // Check if user has 2FA enabled
    let has_2fa = repo.get_two_factor_by_user(&user.id).await?.map(|tf| tf.is_enabled).unwrap_or(false);

    if !has_2fa {
        // Don't reveal whether 2FA is enabled
        return Ok(Status::Ok);
    }

    // Generate and store emergency token hash in database
    let token = repo.create_emergency_token(&user.id).await?;

    // Send emergency 2FA disable email
    let email_service = crate::service::email::EmailService::new(config.email.clone());
    if let Err(e) = email_service
        .send_emergency_2fa_disable_email(&user.email, &user.name, &token, &config.two_factor.frontend_emergency_disable_url)
        .await
    {
        tracing::error!("Failed to send emergency 2FA disable email to {}: {}", user.email, e);
        // Don't fail the request, just log the error
        // In production, you might want to queue this for retry
    } else {
        tracing::info!("Emergency 2FA disable email sent successfully to {}", user.email);
    }

    Ok(Status::Ok)
}

/// Confirm emergency 2FA disable with token from email
#[openapi(tag = "Two-Factor Authentication")]
#[post("/emergency-disable-confirm", data = "<payload>")]
pub async fn emergency_disable_confirm(pool: &State<PgPool>, _rate_limit: RateLimit, payload: Json<EmergencyDisableConfirm>) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Verify the token and get user_id
    let user_id = repo
        .verify_emergency_token(&payload.token)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired emergency disable token.".to_string()))?;

    // Disable 2FA
    repo.disable_two_factor(&user_id).await?;

    // TODO: Invalidate all sessions for this user (force re-login)
    // This will be implemented when we modify the session management

    // TODO: Send confirmation email
    tracing::warn!("Emergency 2FA disable confirmed for user_id: {}", user_id);

    Ok(Status::Ok)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        setup_two_factor,
        verify_two_factor,
        disable_two_factor,
        get_two_factor_status,
        regenerate_backup_codes,
        emergency_disable_request,
        emergency_disable_confirm
    ]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    async fn create_test_user(client: &Client) -> (String, String, String) {
        let payload = serde_json::json!({
            "name": "Test User",
            "email": format!("test.{}@example.com", uuid::Uuid::new_v4()),
            "password": "SecurePassword123!"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id").to_string();
        let user_email = user_json["email"].as_str().expect("user email").to_string();

        (user_id, user_email, "SecurePassword123!".to_string())
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_2fa_setup_generates_qr_and_backup_codes() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Create and login user
        let (_user_id, _user_email, _password) = create_test_user(&client).await;

        // Setup 2FA
        let response = client.post("/api/v1/two-factor/setup").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("setup response body");
        let setup_json: Value = serde_json::from_str(&body).expect("valid setup json");

        // Verify response has required fields
        assert!(setup_json["secret"].is_string());
        assert!(setup_json["qr_code"].is_string());
        assert!(setup_json["backup_codes"].is_array());
        assert_eq!(setup_json["backup_codes"].as_array().unwrap().len(), 10);

        // Verify QR code is a data URL
        let qr_code = setup_json["qr_code"].as_str().unwrap();
        assert!(qr_code.starts_with("data:image/svg+xml;base64,"));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_2fa_verify_with_invalid_code_fails() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Create and login user
        create_test_user(&client).await;

        // Setup 2FA
        let response = client.post("/api/v1/two-factor/setup").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        // Try to verify with invalid code
        let payload = serde_json::json!({
            "code": "000000"
        });

        let response = client
            .post("/api/v1/two-factor/verify")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_2fa_status_returns_correct_state() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Create and login user
        create_test_user(&client).await;

        // Check status before enabling 2FA
        let response = client.get("/api/v1/two-factor/status").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("status response body");
        let status_json: Value = serde_json::from_str(&body).expect("valid status json");

        assert_eq!(status_json["enabled"], false);
        assert_eq!(status_json["has_backup_codes"], false);
        assert_eq!(status_json["backup_codes_remaining"], 0);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_emergency_disable_request_always_returns_success() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;
        config.email.enabled = false; // Disable email for testing

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Request emergency disable for non-existent email
        let payload = serde_json::json!({
            "email": "nonexistent@example.com"
        });

        let response = client
            .post("/api/v1/two-factor/emergency-disable-request")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        // Should still return success to prevent email enumeration
        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_disable_2fa_requires_password_and_code() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Create and login user
        create_test_user(&client).await;

        // Try to disable 2FA without setting it up first
        let payload = serde_json::json!({
            "password": "SecurePassword123!",
            "code": "123456"
        });

        let response = client
            .delete("/api/v1/two-factor/disable")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        // Should fail because 2FA is not enabled
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_regenerate_backup_codes_requires_valid_code() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Create and login user
        create_test_user(&client).await;

        // Try to regenerate backup codes without 2FA enabled
        let payload = serde_json::json!({
            "code": "123456"
        });

        let response = client
            .post("/api/v1/two-factor/regenerate-backup-codes")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        // Should fail because 2FA is not enabled
        assert_eq!(response.status(), Status::BadRequest);
    }
}
