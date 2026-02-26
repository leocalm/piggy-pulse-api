use rocket::figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TwoFactorConfig {
    /// Hex-encoded 32-byte encryption key for TOTP secrets (generate with: openssl rand -hex 32)
    #[serde(default = "default_encryption_key")]
    pub encryption_key: String,
    /// Issuer name shown in authenticator apps (e.g., "PiggyPulse")
    #[serde(default = "default_issuer_name")]
    pub issuer_name: String,
    /// Frontend URL for emergency 2FA disable confirmation
    #[serde(default = "default_emergency_disable_url")]
    pub frontend_emergency_disable_url: String,
}

pub const INSECURE_DEFAULT_TWO_FACTOR_ENCRYPTION_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn default_encryption_key() -> String {
    // WARNING: This is a placeholder key for development only
    // Generate a secure key with: openssl rand -hex 32
    // Set PIGGY_PULSE_TWO_FACTOR__ENCRYPTION_KEY environment variable in production
    INSECURE_DEFAULT_TWO_FACTOR_ENCRYPTION_KEY.to_string()
}

fn default_issuer_name() -> String {
    "PiggyPulse".to_string()
}

fn default_emergency_disable_url() -> String {
    "http://localhost:5173/auth/emergency-2fa-disable".to_string()
}

impl TwoFactorConfig {
    pub fn encryption_key_is_default(&self) -> bool {
        self.encryption_key.eq_ignore_ascii_case(INSECURE_DEFAULT_TWO_FACTOR_ENCRYPTION_KEY)
    }

    pub fn parse_encryption_key(&self) -> Result<[u8; 32], String> {
        let encryption_key_bytes = hex::decode(&self.encryption_key).map_err(|e| format!("Invalid encryption key configuration: {}", e))?;

        if encryption_key_bytes.len() != 32 {
            return Err("Encryption key must be exactly 32 bytes (64 hex chars)".to_string());
        }

        let mut encryption_key = [0u8; 32];
        encryption_key.copy_from_slice(&encryption_key_bytes);
        Ok(encryption_key)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub session: SessionConfig,
    pub api: ApiConfig,
    pub email: EmailConfig,
    pub password_reset: PasswordResetConfig,
    pub two_factor: TwoFactorConfig,
    #[serde(default)]
    pub login_rate_limit: LoginRateLimitConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    pub acquire_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub json_format: bool,
    pub slow_request_ms: u64,
    pub slow_query_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimitConfig {
    pub read_limit: u32,
    pub mutation_limit: u32,
    pub auth_limit: u32,
    pub window_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub require_client_ip: bool,
    pub use_forwarded_ip: bool,
    pub forwarded_ip_header: String,
    pub backend: RateLimitBackend,
    pub redis_url: String,
    pub redis_key_prefix: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitBackend {
    Redis,
    InMemory,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SessionConfig {
    pub ttl_seconds: i64,
    pub cookie_secure: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub base_path: String,
    pub additional_base_paths: Vec<String>,
    pub expose_docs: bool,
    pub expose_swagger_ui: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub from_name: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PasswordResetConfig {
    pub token_ttl_seconds: i64,
    pub max_attempts_per_hour: u32,
    pub frontend_reset_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRateLimitConfig {
    #[serde(default = "default_free_attempts")]
    pub free_attempts: i32,

    #[serde(default = "default_delay_seconds")]
    pub delay_seconds: Vec<i64>,

    #[serde(default = "default_lockout_attempts")]
    pub lockout_attempts: i32,

    #[serde(default = "default_lockout_duration_minutes")]
    pub lockout_duration_minutes: i64,

    #[serde(default = "default_enable_email_unlock")]
    pub enable_email_unlock: bool,

    #[serde(default = "default_notify_user_on_lock")]
    pub notify_user_on_lock: bool,

    #[serde(default = "default_notify_admin_on_lock")]
    pub notify_admin_on_lock: bool,

    pub admin_email: Option<String>,

    #[serde(default = "default_high_failure_threshold")]
    pub high_failure_threshold: i32,
}

impl Default for LoginRateLimitConfig {
    fn default() -> Self {
        Self {
            free_attempts: default_free_attempts(),
            delay_seconds: default_delay_seconds(),
            lockout_attempts: default_lockout_attempts(),
            lockout_duration_minutes: default_lockout_duration_minutes(),
            enable_email_unlock: default_enable_email_unlock(),
            notify_user_on_lock: default_notify_user_on_lock(),
            notify_admin_on_lock: default_notify_admin_on_lock(),
            admin_email: None,
            high_failure_threshold: default_high_failure_threshold(),
        }
    }
}

fn default_free_attempts() -> i32 { 3 }
fn default_delay_seconds() -> Vec<i64> { vec![5, 30, 60] }
fn default_lockout_attempts() -> i32 { 7 }
fn default_lockout_duration_minutes() -> i64 { 60 }
fn default_enable_email_unlock() -> bool { true }
fn default_notify_user_on_lock() -> bool { true }
fn default_notify_admin_on_lock() -> bool { true }
fn default_high_failure_threshold() -> i32 { 20 }

pub const DEFAULT_API_BASE_PATH: &str = "/api/v1";

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/piggy_pulse_db".to_string(),
            max_connections: 16,
            min_connections: 4,
            connection_timeout: 5,
            acquire_timeout: 5,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8000,
            address: "127.0.0.1".to_string(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            slow_request_ms: 500,
            slow_query_ms: 100,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            read_limit: 300,
            mutation_limit: 60,
            auth_limit: 10,
            window_seconds: 60,
            cleanup_interval_seconds: 60,
            require_client_ip: true,
            use_forwarded_ip: false,
            forwarded_ip_header: "x-forwarded-for".to_string(),
            backend: RateLimitBackend::InMemory,
            redis_url: "redis://127.0.0.1:6379/0".to_string(),
            redis_key_prefix: "piggy-pulse:rate_limit:".to_string(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 60 * 60 * 24 * 30,
            cookie_secure: true,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_path: DEFAULT_API_BASE_PATH.to_string(),
            additional_base_paths: Vec::new(),
            expose_docs: false,
            expose_swagger_ui: false,
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "".to_string(),
            smtp_password: "".to_string(),
            from_address: "noreply@piggy-pulse.com".to_string(),
            from_name: "PiggyPulse".to_string(),
            enabled: false, // Disabled by default for safety
        }
    }
}

impl Default for PasswordResetConfig {
    fn default() -> Self {
        Self {
            token_ttl_seconds: 900, // 15 minutes
            max_attempts_per_hour: 3,
            frontend_reset_url: "http://localhost:3000/reset-password".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in priority order:
    /// 1. PiggyPulse.toml (base configuration file)
    /// 2. Environment variables (prefixed with PIGGY_PULSE_)
    /// 3. DATABASE_URL environment variable (fallback/backwards-compat)
    pub fn load() -> Result<Self, Box<figment::Error>> {
        let mut cfg: Self = Figment::new()
            // Start with defaults.
            .merge(Toml::string(&toml::to_string(&Config::default()).unwrap()))
            // Layer on PiggyPulse.toml if it exists.
            .merge(Toml::file("PiggyPulse.toml"))
            // Layer on environment variables (e.g., PIGGY_PULSE_DATABASE__URL).
            .merge(Env::prefixed("PIGGY_PULSE_").split("__"))
            .extract()?;

        // Backwards-compat: DATABASE_URL overrides the default/TOML value, but not an explicitly
        // set PIGGY_PULSE_DATABASE__URL.
        if std::env::var_os("PIGGY_PULSE_DATABASE__URL").is_none()
            && let Ok(url) = std::env::var("DATABASE_URL")
        {
            cfg.database.url = url;
        }

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::{INSECURE_DEFAULT_TWO_FACTOR_ENCRYPTION_KEY, TwoFactorConfig};

    #[test]
    fn parse_encryption_key_accepts_32_byte_hex() {
        let config = TwoFactorConfig {
            encryption_key: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            issuer_name: "PiggyPulse".to_string(),
            frontend_emergency_disable_url: "http://localhost".to_string(),
        };

        let parsed = config.parse_encryption_key().expect("valid key should parse");
        assert_eq!(parsed.len(), 32);
    }

    #[test]
    fn parse_encryption_key_rejects_wrong_length() {
        let config = TwoFactorConfig {
            encryption_key: "abcd".to_string(),
            issuer_name: "PiggyPulse".to_string(),
            frontend_emergency_disable_url: "http://localhost".to_string(),
        };

        let err = config.parse_encryption_key().expect_err("short key should fail");
        assert_eq!(err, "Encryption key must be exactly 32 bytes (64 hex chars)");
    }

    #[test]
    fn encryption_key_default_detection_is_case_insensitive() {
        let config = TwoFactorConfig {
            encryption_key: INSECURE_DEFAULT_TWO_FACTOR_ENCRYPTION_KEY.to_uppercase(),
            issuer_name: "PiggyPulse".to_string(),
            frontend_emergency_disable_url: "http://localhost".to_string(),
        };

        assert!(config.encryption_key_is_default());
    }
}

#[cfg(test)]
mod rate_limit_tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = LoginRateLimitConfig::default();
        assert_eq!(config.free_attempts, 3);
        assert_eq!(config.delay_seconds, vec![5, 30, 60]);
        assert_eq!(config.lockout_attempts, 7);
        assert_eq!(config.lockout_duration_minutes, 60);
    }
}
