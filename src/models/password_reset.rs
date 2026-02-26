use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

/// Password reset record stored in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordReset {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Request to initiate password reset (sent by user)
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct PasswordResetRequest {
    #[validate(email)]
    pub email: String,
}

/// Request to validate a password reset token
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct PasswordResetValidateRequest {
    #[validate(length(equal = 64))]
    pub token: String,
}

/// Request to confirm password reset with new password
#[derive(Debug, Deserialize, Validate, JsonSchema)]
pub struct PasswordResetConfirmRequest {
    #[validate(length(equal = 64))]
    pub token: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "crate::models::user::validate_password_strength"))]
    pub new_password: String,
}

/// Response for password reset request (always success to prevent email enumeration)
#[derive(Debug, Serialize, JsonSchema)]
pub struct PasswordResetResponse {
    pub message: String,
}

/// Response for password reset validation
#[derive(Debug, Serialize, JsonSchema)]
pub struct PasswordResetValidateResponse {
    pub valid: bool,
    pub email: Option<String>, // Only returned if valid
}

impl PasswordReset {
    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the token has been used
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if the token is still valid (not expired and not used)
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}
