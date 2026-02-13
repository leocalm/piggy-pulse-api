use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;

/// Database model for two-factor authentication
#[derive(Debug, sqlx::FromRow)]
pub struct TwoFactorAuth {
    pub id: Uuid,
    pub user_id: Uuid,
    pub encrypted_secret: String,
    pub encryption_nonce: String,
    pub is_enabled: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response when setting up 2FA (contains sensitive data - send once only)
#[derive(Debug, Serialize, JsonSchema)]
pub struct TwoFactorSetupResponse {
    /// The TOTP secret in base32 format (for manual entry)
    pub secret: String,
    /// QR code as a data URL (image/png base64)
    pub qr_code: String,
    /// One-time backup codes (10 codes, shown only once)
    pub backup_codes: Vec<String>,
}

/// Request to verify and enable 2FA
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TwoFactorVerifyRequest {
    /// 6-digit TOTP code from authenticator app
    pub code: String,
}

/// Request to disable 2FA (requires password + current code)
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TwoFactorDisableRequest {
    /// User's current password
    pub password: String,
    /// Current 2FA code (TOTP or backup code)
    pub code: String,
}

/// Request to regenerate backup codes
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TwoFactorRegenerateRequest {
    /// Current 2FA code to authorize regeneration
    pub code: String,
}

/// 2FA status for current user
#[derive(Debug, Serialize, JsonSchema)]
pub struct TwoFactorStatus {
    /// Whether 2FA is enabled and verified
    pub enabled: bool,
    /// Whether user has any backup codes
    pub has_backup_codes: bool,
    /// Number of unused backup codes remaining
    pub backup_codes_remaining: i32,
}

/// Backup code database model
#[derive(Debug, sqlx::FromRow)]
pub struct BackupCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Rate limit database model
#[derive(Debug, sqlx::FromRow)]
pub struct TwoFactorRateLimit {
    pub id: Uuid,
    pub user_id: Uuid,
    pub failed_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_attempt_at: DateTime<Utc>,
}

/// Emergency disable token model
#[derive(Debug, sqlx::FromRow)]
pub struct EmergencyToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Request to initiate emergency 2FA disable
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmergencyDisableRequest {
    /// Email address of the account
    pub email: String,
}

/// Request to confirm emergency disable
#[derive(Debug, Deserialize, JsonSchema)]
pub struct EmergencyDisableConfirm {
    /// Token from email
    pub token: String,
}
