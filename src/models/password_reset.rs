use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use validator::Validate;

/// Password reset record stored in the database
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PasswordReset {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Security audit log entry
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SecurityAuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
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

/// Event types for security audit log
pub mod audit_events {
    pub const PASSWORD_RESET_REQUESTED: &str = "password_reset_requested";
    pub const PASSWORD_RESET_TOKEN_VALIDATED: &str = "password_reset_token_validated";
    pub const PASSWORD_RESET_COMPLETED: &str = "password_reset_completed";
    pub const PASSWORD_RESET_FAILED: &str = "password_reset_failed";
    pub const PASSWORD_RESET_TOKEN_EXPIRED: &str = "password_reset_token_expired";
    pub const PASSWORD_RESET_TOKEN_INVALID: &str = "password_reset_token_invalid";
}
