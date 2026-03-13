#![allow(unused)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ===== User =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub currency: String,
    pub two_factor_enabled: bool,
}

// ===== Login =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorChallengeResponse {
    pub requires_two_factor: bool,
    pub two_factor_token: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedResponse {
    pub requires_two_factor: bool,
    pub user: UserResponse,
    pub token: Option<String>,
}

/// Discriminated union on `requiresTwoFactor` (boolean).
/// Serialized as untagged — the `requiresTwoFactor` field in each variant acts as the discriminator.
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum LoginResponse {
    TwoFactorChallenge(TwoFactorChallengeResponse),
    Authenticated(AuthenticatedResponse),
}

// ===== Register =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
    #[validate(length(min = 1))]
    pub name: String,
}

// ===== 2FA Complete (after challenge) =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorCompleteRequest {
    #[validate(length(min = 1))]
    pub two_factor_token: String,
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}

// ===== Token Refresh =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResponse {
    pub token: String,
}

// ===== Forgot / Reset Password =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 1))]
    pub password: String,
}

// ===== 2FA Management =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorEnableResponse {
    pub secret: String,
    pub qr_code_uri: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorVerifyRequest {
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorDisableRequest {
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}

// ===== 2FA Status =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
    pub has_backup_codes: bool,
    pub backup_codes_remaining: i32,
}

// ===== Recovery / Backup Codes =====

pub type BackupCodesResponse = Vec<String>;

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateBackupCodesRequest {
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}

// ===== Emergency Disable =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyDisableRequestBody {
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyDisableConfirmRequest {
    #[validate(length(min = 1))]
    pub token: String,
}

// ===== Change Password =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(length(min = 1))]
    pub new_password: String,
}
