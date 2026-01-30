use rocket::figment::{Figment, providers::{Env, Format, Toml}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in priority order:
    /// 1. Budget.toml (base configuration file)
    /// 2. Environment variables (prefixed with BUDGET_)
    /// 3. DATABASE_URL environment variable (for backwards compatibility)
    pub fn load() -> Result<Self, figment::Error> {
        let figment = Figment::new()
            // Start with defaults
            .merge(Toml::string(&toml::to_string(&Config::default()).unwrap()).nested())
            // Layer on Budget.toml if it exists
            .merge(Toml::file("Budget.toml").nested())
            // Layer on environment variables (e.g., BUDGET_DATABASE_URL)
            .merge(Env::prefixed("BUDGET_").split("_"))
            // Special case: DATABASE_URL for backwards compatibility
            .merge(Env::raw().only(&["DATABASE_URL"]).map(|_| "database.url".into()));

        figment.extract()
    }
}
