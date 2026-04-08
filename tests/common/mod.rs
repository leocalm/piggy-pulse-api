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
pub const V2_BASE: &str = "/v2";
#[allow(dead_code)]
pub const TEST_PASSWORD: &str = "CorrectHorseBatteryStaple!2026";

/// Clear login rate limits from the test database to prevent progressive backoff
/// from interfering with test runs.
#[allow(dead_code)]
pub async fn clear_login_rate_limits() {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string());
    let pool = sqlx::PgPool::connect(&url).await.expect("connect to test db");
    sqlx::query("DELETE FROM login_rate_limits")
        .execute(&pool)
        .await
        .expect("clear login_rate_limits");
}

#[allow(dead_code)]
pub fn test_config() -> Config {
    let mut config = Config::default();
    config.database.url = std::env::var("DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string());
    config.database.max_connections = 2;
    config.database.min_connections = 1;
    config.rate_limit.require_client_ip = false;
    config.session.cookie_secure = false;
    // Set a valid 2FA encryption key for tests (insecure, debug-only)
    config.two_factor.encryption_key = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    config
}

#[allow(dead_code)]
pub async fn test_client() -> Client {
    Client::tracked(build_rocket(test_config())).await.expect("valid rocket instance")
}
