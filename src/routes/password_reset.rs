use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::AuthRateLimit;
use crate::models::password_reset::{
    PasswordResetConfirmRequest, PasswordResetRequest, PasswordResetResponse, PasswordResetValidateRequest, PasswordResetValidateResponse, audit_events,
};
use crate::service::email::EmailService;
use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, post};
use rocket_okapi::openapi;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use validator::Validate;

/// Request a password reset (Step 1: Send email with reset token)
#[openapi(tag = "Password Reset")]
#[post("/password-reset/request", data = "<payload>")]
pub async fn request_password_reset(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: AuthRateLimit,
    payload: Json<PasswordResetRequest>,
) -> Result<Json<PasswordResetResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Look up user by email
    match repo.get_user_by_email(&payload.email).await? {
        Some(user) => {
            // Check rate limiting: max attempts per user
            let since = Utc::now() - chrono::Duration::hours(1);
            let attempts = repo.count_password_reset_attempts(&user.id, since).await?;

            if attempts >= config.password_reset.max_attempts_per_hour as i64 {
                // Log the rate limit violation
                let _ = repo
                    .create_security_audit_log(
                        Some(&user.id),
                        audit_events::PASSWORD_RESET_FAILED,
                        false,
                        None, // Could extract IP from request
                        None, // Could extract user agent from request
                        Some(serde_json::json!({
                            "reason": "rate_limit_exceeded",
                            "attempts": attempts
                        })),
                    )
                    .await;

                // Still return success to prevent email enumeration
                return Ok(Json(PasswordResetResponse {
                    message: "If your email address exists in our system, you will receive a password reset link shortly.".to_string(),
                }));
            }

            // Generate reset token
            let (plain_token, token_hash) = PostgresRepository::generate_reset_token();

            // Create password reset record
            let expires_at = Utc::now() + chrono::Duration::seconds(config.password_reset.token_ttl_seconds);

            repo.create_password_reset(&user.id, &token_hash, expires_at, None, None).await?;

            // Log the request
            let _ = repo
                .create_security_audit_log(
                    Some(&user.id),
                    audit_events::PASSWORD_RESET_REQUESTED,
                    true,
                    None,
                    None,
                    Some(serde_json::json!({"email": &payload.email})),
                )
                .await;

            // Send email with reset link
            let email_service = EmailService::new(config.email.clone());
            if let Err(e) = email_service
                .send_password_reset_email(&user.email, &user.name, &plain_token, &config.password_reset.frontend_reset_url)
                .await
            {
                tracing::error!("Failed to send password reset email: {}", e);
                // Don't fail the request, just log the error
                // In production, you might want to queue this for retry
            }
        }
        None => {
            // User not found - perform timing-constant fake work to prevent email enumeration
            PostgresRepository::dummy_verify("fake_password");

            // Log the attempt with no user_id
            let _ = repo
                .create_security_audit_log(
                    None,
                    audit_events::PASSWORD_RESET_FAILED,
                    false,
                    None,
                    None,
                    Some(serde_json::json!({
                        "reason": "user_not_found",
                        "email": &payload.email
                    })),
                )
                .await;
        }
    }

    // Always return success to prevent email enumeration
    Ok(Json(PasswordResetResponse {
        message: "If your email address exists in our system, you will receive a password reset link shortly.".to_string(),
    }))
}

/// Validate a password reset token (Step 2: Check if token is valid)
#[openapi(tag = "Password Reset")]
#[post("/password-reset/validate", data = "<payload>")]
pub async fn validate_password_reset_token(
    pool: &State<PgPool>,
    _rate_limit: AuthRateLimit,
    payload: Json<PasswordResetValidateRequest>,
) -> Result<Json<PasswordResetValidateResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Hash the token to look it up
    let mut hasher = Sha256::new();
    hasher.update(&payload.token);
    let token_hash = hex::encode(hasher.finalize());

    match repo.get_password_reset_by_token(&token_hash).await? {
        Some(reset) => {
            if !reset.is_valid() {
                // Token is expired or already used
                let reason = if reset.is_expired() {
                    audit_events::PASSWORD_RESET_TOKEN_EXPIRED
                } else {
                    audit_events::PASSWORD_RESET_TOKEN_INVALID
                };

                let _ = repo.create_security_audit_log(Some(&reset.user_id), reason, false, None, None, None).await;

                return Ok(Json(PasswordResetValidateResponse { valid: false, email: None }));
            }

            // Token is valid, fetch user email
            if let Some(user) = repo.get_user_by_id(&reset.user_id).await? {
                let _ = repo
                    .create_security_audit_log(Some(&user.id), audit_events::PASSWORD_RESET_TOKEN_VALIDATED, true, None, None, None)
                    .await;

                return Ok(Json(PasswordResetValidateResponse {
                    valid: true,
                    email: Some(user.email),
                }));
            }

            Ok(Json(PasswordResetValidateResponse { valid: false, email: None }))
        }
        None => {
            // Token not found
            let _ = repo
                .create_security_audit_log(None, audit_events::PASSWORD_RESET_TOKEN_INVALID, false, None, None, None)
                .await;

            Ok(Json(PasswordResetValidateResponse { valid: false, email: None }))
        }
    }
}

/// Confirm password reset and set new password (Step 3: Complete the reset)
#[openapi(tag = "Password Reset")]
#[post("/password-reset/confirm", data = "<payload>")]
pub async fn confirm_password_reset(pool: &State<PgPool>, _rate_limit: AuthRateLimit, payload: Json<PasswordResetConfirmRequest>) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Hash the token to look it up
    let mut hasher = Sha256::new();
    hasher.update(&payload.token);
    let token_hash = hex::encode(hasher.finalize());

    match repo.get_password_reset_by_token(&token_hash).await? {
        Some(reset) => {
            if !reset.is_valid() {
                // Token is expired or already used
                let reason = if reset.is_expired() {
                    "Token has expired"
                } else {
                    "Token has already been used"
                };

                let _ = repo
                    .create_security_audit_log(
                        Some(&reset.user_id),
                        audit_events::PASSWORD_RESET_FAILED,
                        false,
                        None,
                        None,
                        Some(serde_json::json!({"reason": reason})),
                    )
                    .await;

                return Err(AppError::BadRequest(reason.to_string()));
            }

            // Update the user's password
            repo.update_user_password(&reset.user_id, &payload.new_password).await?;

            // Mark the token as used
            repo.mark_password_reset_used(&reset.id).await?;

            // Invalidate all existing sessions for security
            let sessions_invalidated = repo.invalidate_all_user_sessions(&reset.user_id).await?;

            // Delete all other password reset tokens for this user
            repo.delete_password_resets_for_user(&reset.user_id).await?;

            // Log successful password reset
            let _ = repo
                .create_security_audit_log(
                    Some(&reset.user_id),
                    audit_events::PASSWORD_RESET_COMPLETED,
                    true,
                    None,
                    None,
                    Some(serde_json::json!({"sessions_invalidated": sessions_invalidated})),
                )
                .await;

            tracing::info!("Password reset completed successfully for user {}", reset.user_id);

            Ok(Status::Ok)
        }
        None => {
            // Token not found
            let _ = repo
                .create_security_audit_log(
                    None,
                    audit_events::PASSWORD_RESET_FAILED,
                    false,
                    None,
                    None,
                    Some(serde_json::json!({"reason": "invalid_token"})),
                )
                .await;

            Err(AppError::BadRequest("Invalid or expired reset token".to_string()))
        }
    }
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![request_password_reset, validate_password_reset_token, confirm_password_reset]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_request_password_reset_always_returns_success() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;
        config.email.enabled = false; // Disable email for testing

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Request reset for non-existent email
        let payload = serde_json::json!({
            "email": "nonexistent@example.com"
        });

        let response = client
            .post("/api/v1/password-reset/request")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        // Should still return success to prevent email enumeration
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("response body");
        assert!(body.contains("If your email address exists"));
    }
}
