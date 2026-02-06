use figment::providers::Env;
use rocket::figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
    pub rate_limit: RateLimitConfig,
    pub api: ApiConfig,
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
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub base_path: String,
    pub additional_base_paths: Vec<String>,
    pub enable_swagger: bool,
}

pub const DEFAULT_API_BASE_PATH: &str = "/api/v1";

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/budget_db".to_string(),
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
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_path: DEFAULT_API_BASE_PATH.to_string(),
            additional_base_paths: Vec::new(),
            enable_swagger: false,
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in priority order:
    /// 1. Budget.toml (base configuration file)
    /// 2. Environment variables (prefixed with BUDGET_)
    pub fn load() -> Result<Self, Box<figment::Error>> {
        let figment = Figment::new()
            // Start with defaults
            .merge(Toml::string(&toml::to_string(&Config::default()).unwrap()))
            // Layer on Budget.toml if it exists
            .merge(Toml::file("Budget.toml"))
            // Layer on environment variables (e.g., BUDGET_DATABASE_URL)
            .merge(Env::prefixed("BUDGET_").split("__"))
            .extract()?;

        Ok(figment)
    }
}
