/// Event types for security audit log
pub mod audit_events {
    // Authentication events
    pub const LOGIN_SUCCESS: &str = "login_success";
    pub const LOGIN_FAILED: &str = "login_failed";
    pub const LOGOUT: &str = "logout";
    pub const SESSION_EXPIRED: &str = "session_expired";

    // Two-factor authentication events
    pub const TWO_FACTOR_ENABLED: &str = "2fa_enabled";
    pub const TWO_FACTOR_DISABLED: &str = "2fa_disabled";
    pub const TWO_FACTOR_BACKUP_USED: &str = "2fa_backup_used";

    // Account events
    pub const PASSWORD_CHANGED: &str = "password_changed";
    pub const ACCOUNT_UPDATED: &str = "account_updated";

    // Password reset events (moved from password_reset.rs)
    pub const PASSWORD_RESET_REQUESTED: &str = "password_reset_requested";
    pub const PASSWORD_RESET_TOKEN_VALIDATED: &str = "password_reset_token_validated";
    pub const PASSWORD_RESET_COMPLETED: &str = "password_reset_completed";
    pub const PASSWORD_RESET_FAILED: &str = "password_reset_failed";
    pub const PASSWORD_RESET_TOKEN_EXPIRED: &str = "password_reset_token_expired";
    pub const PASSWORD_RESET_TOKEN_INVALID: &str = "password_reset_token_invalid";
}
