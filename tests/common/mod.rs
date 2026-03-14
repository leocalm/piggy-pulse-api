#[allow(dead_code)]
pub mod assertions;
#[allow(dead_code)]
pub mod auth;
#[allow(dead_code)]
pub mod entities;

use piggy_pulse::{Config, build_rocket};
use rocket::local::asynchronous::Client;

#[allow(dead_code)]
pub const TEST_DB_URL: &str = "postgres://postgres:test_password@127.0.0.1:5433/piggy_pulse_test";
#[allow(dead_code)]
pub const V2_BASE: &str = "/api/v2";
#[allow(dead_code)]
pub const V1_BASE: &str = "/api/v1";
#[allow(dead_code)]
pub const TEST_PASSWORD: &str = "CorrectHorseBatteryStaple!2026";

#[allow(dead_code)]
pub fn test_config() -> Config {
    let mut config = Config::default();
    config.database.url = std::env::var("DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string());
    config.database.max_connections = 2;
    config.database.min_connections = 1;
    config.rate_limit.require_client_ip = false;
    config.session.cookie_secure = false;
    config
}

#[allow(dead_code)]
pub async fn test_client() -> Client {
    Client::tracked(build_rocket(test_config())).await.expect("valid rocket instance")
}
