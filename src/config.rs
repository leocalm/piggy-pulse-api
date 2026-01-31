use rocket::figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub cors: CorsConfig,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
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

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allow_credentials: false,
        }
    }
}

impl Config {
    /// Load configuration from multiple sources in priority order:
    /// 1. Budget.toml (base configuration file)
    /// 2. Environment variables (prefixed with BUDGET_)
    /// 3. DATABASE_URL environment variable (fallback/backwards-compat)
    pub fn load() -> Result<Self, figment::Error> {
        let mut cfg: Self = Figment::new()
            // Start with defaults.
            .merge(Toml::string(&toml::to_string(&Config::default()).unwrap()))
            // Layer on Budget.toml if it exists.
            .merge(Toml::file("Budget.toml"))
            // Layer on environment variables (e.g., BUDGET_DATABASE__URL).
            .merge(Env::prefixed("BUDGET_").split("__"))
            .extract()?;

        // Backwards-compat: DATABASE_URL overrides the default/TOML value, but not an explicitly
        // set BUDGET_DATABASE__URL.
        if std::env::var_os("BUDGET_DATABASE__URL").is_none() {
            if let Ok(url) = std::env::var("DATABASE_URL") {
                cfg.database.url = url;
            }
        }

        Ok(cfg)
    }
}
