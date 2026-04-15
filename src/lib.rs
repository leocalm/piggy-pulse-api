mod auth;
mod config;
mod cron_tasks;
pub mod crypto;
mod database;
mod db;
mod dto;
mod error;
mod middleware;
mod models;
mod routes;
mod service;
pub mod session_dek;

#[cfg(test)]
pub mod test_utils;

pub use config::Config;
pub use cron_tasks::{GeneratePeriodsResult, cleanup_expired_tokens, generate_periods};

use crate::db::stage_db;
use crate::middleware::RequestLogger;
use crate::routes as app_routes;
use rocket::{Build, Rocket, catchers, http::Method};
use rocket_cors::{AllowedOrigins, CorsOptions};
use tracing_subscriber::EnvFilter;

fn init_tracing(log_level: &str, json_format: bool) {
    use std::sync::Once;
    static TRACING_INIT: Once = Once::new();

    TRACING_INIT.call_once(|| {
        // Configure logging with environment variable support
        // RUST_LOG environment variable can be used for fine-grained control per module:
        // Examples:
        //   RUST_LOG=debug                    - Set all to debug
        //   RUST_LOG=piggy_pulse=debug             - Set piggy_pulse crate to debug
        //   RUST_LOG=piggy_pulse::routes=trace     - Set specific module to trace
        //   RUST_LOG=info,piggy_pulse::routes=debug - Global info, routes at debug
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(false);

        if json_format {
            subscriber.json().init();
        } else {
            subscriber.init();
        }
    });
}

fn ensure_rocket_secret_key() {
    // Only require ROCKET_SECRET_KEY for non-debug profiles (e.g., release builds).
    let profile = std::env::var("ROCKET_PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile != "debug" && std::env::var("ROCKET_SECRET_KEY").is_err() {
        panic!(
            "ROCKET_SECRET_KEY is required for profile '{}' — generate one with: openssl rand -base64 32",
            profile
        );
    }
}

fn ensure_two_factor_encryption_key(two_factor_config: &config::TwoFactorConfig) {
    // Only enforce strict 2FA key requirements for non-debug profiles (e.g., staging/production).
    let profile = std::env::var("ROCKET_PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile == "debug" {
        return;
    }

    if two_factor_config.encryption_key_is_default() {
        panic!(
            "PIGGY_PULSE_TWO_FACTOR__ENCRYPTION_KEY must be set for profile '{}' and cannot use the insecure default. Generate one with: openssl rand -hex 32",
            profile
        );
    }

    if let Err(err) = two_factor_config.parse_encryption_key() {
        panic!("Invalid PIGGY_PULSE_TWO_FACTOR__ENCRYPTION_KEY for profile '{}': {}", profile, err);
    }
}

fn ensure_cookie_secure(session_config: &config::SessionConfig) {
    let profile = std::env::var("ROCKET_PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile == "debug" {
        return;
    }

    if !session_config.cookie_secure {
        panic!(
            "PIGGY_PULSE_SESSION__COOKIE_SECURE must be true for profile '{}'. Insecure cookies are only allowed in debug mode.",
            profile
        );
    }
}

fn build_cors(cors_config: &config::CorsConfig) -> CorsOptions {
    if cors_config.allowed_origins.is_empty() {
        if cors_config.allow_credentials {
            panic!("Invalid CORS configuration: allow_credentials requires explicit allowed_origins.");
        }
        // Secure default: no CORS origins are allowed unless explicitly configured.
        return CorsOptions {
            allowed_origins: AllowedOrigins::some_exact::<&str>(&[]),
            allowed_methods: vec![
                Method::Get,
                Method::Post,
                Method::Put,
                Method::Delete,
                Method::Patch,
                Method::Options,
                Method::Head,
            ]
            .into_iter()
            .map(From::from)
            .collect(),
            allowed_headers: rocket_cors::AllowedHeaders::some(&["Content-Type", "Authorization", "Accept"]),
            allow_credentials: false,
            ..Default::default()
        };
    }

    let is_wildcard = cors_config.allowed_origins.len() == 1 && cors_config.allowed_origins[0] == "*";

    // Validate that wildcard origins are not combined with credentials
    if is_wildcard && cors_config.allow_credentials {
        panic!(
            "Invalid CORS configuration: Cannot use wildcard origins (*) with credentials enabled. \
            Either set specific origins or disable credentials."
        );
    }

    let allowed_origins = if is_wildcard {
        AllowedOrigins::all()
    } else {
        AllowedOrigins::some_exact(&cors_config.allowed_origins.iter().map(String::as_str).collect::<Vec<_>>())
    };

    CorsOptions {
        allowed_origins,
        allowed_methods: vec![
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Patch,
            Method::Options,
            Method::Head,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: rocket_cors::AllowedHeaders::some(&["Content-Type", "Authorization", "Accept"]),
        allow_credentials: cors_config.allow_credentials,
        ..Default::default()
    }
}

fn normalize_base_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return config::DEFAULT_API_BASE_PATH.to_string();
    }

    let mut normalized = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    };

    while normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }

    normalized
}

fn join_base_path(base_path: &str, path: &str) -> String {
    let base = base_path.trim_end_matches('/');
    let suffix = path.trim_start_matches('/');

    if base.is_empty() {
        format!("/{}", suffix)
    } else {
        format!("{}/{}", base, suffix)
    }
}

/// Mount v2 route handlers (spec-first, no rocket_okapi / OpenAPI generation).
/// V2 routes are not included in Swagger UI — the OpenAPI spec is maintained externally.
fn mount_v2_routes(mut rocket: Rocket<Build>, base_path: &str) -> Rocket<Build> {
    // Auth (multiple route groups)
    rocket = rocket.mount(join_base_path(base_path, "auth"), app_routes::v2::auth::routes());
    rocket = rocket.mount(join_base_path(base_path, "auth/2fa"), app_routes::v2::auth::two_factor_routes());
    // Resources
    rocket = rocket.mount(join_base_path(base_path, "accounts"), app_routes::v2::accounts::routes());
    rocket = rocket.mount(join_base_path(base_path, "categories"), app_routes::v2::categories::routes());
    rocket = rocket.mount(join_base_path(base_path, "vendors"), app_routes::v2::vendors::routes());
    rocket = rocket.mount(join_base_path(base_path, "periods"), app_routes::v2::periods::routes());
    rocket = rocket.mount(join_base_path(base_path, "targets"), app_routes::v2::targets::routes());
    rocket = rocket.mount(join_base_path(base_path, "transactions"), app_routes::v2::transactions::routes());
    // Settings (multiple route groups)
    rocket = rocket.mount(join_base_path(base_path, "settings"), app_routes::v2::settings::routes());
    rocket = rocket.mount(join_base_path(base_path, "settings/sessions"), app_routes::v2::settings::session_routes());
    rocket = rocket.mount(join_base_path(base_path, "settings/export"), app_routes::v2::settings::export_routes());
    rocket = rocket.mount(join_base_path(base_path, "settings/import"), app_routes::v2::settings::import_routes());
    // Dashboard, reference data, system
    rocket = rocket.mount(join_base_path(base_path, "subscriptions"), app_routes::v2::subscriptions::routes());
    rocket = rocket.mount(join_base_path(base_path, "currencies"), app_routes::v2::currencies::routes());
    rocket = rocket.mount(join_base_path(base_path, "onboarding"), app_routes::v2::onboarding::routes());
    rocket = rocket.mount(join_base_path(base_path, "health"), app_routes::v2::health::routes());
    rocket = rocket.mount(join_base_path(base_path, ""), app_routes::v2::unlock::routes());
    rocket
}

// V1 API routes have been removed. All endpoints are now served via V2 routes.

pub fn build_rocket(config: Config) -> Rocket<Build> {
    dotenvy::dotenv().ok();
    init_tracing(&config.logging.level, config.logging.json_format);
    ensure_rocket_secret_key();
    ensure_two_factor_encryption_key(&config.two_factor);
    ensure_cookie_secure(&config.session);

    let cors = build_cors(&config.cors).to_cors().expect("Failed to create CORS fairing");

    let base_path = normalize_base_path(&config.api.base_path);

    let mut rocket = rocket::build()
        .manage(config.clone())
        .manage(session_dek::SessionDekStore::new())
        .attach(cors)
        .attach(RequestLogger)
        .attach(stage_db(config.database, config.logging.slow_query_ms));

    rocket = mount_v2_routes(rocket, &base_path);

    let all_catchers = catchers![
        app_routes::error::bad_request,
        app_routes::error::not_found,
        app_routes::error::conflict,
        app_routes::error::unprocessable_entity,
        app_routes::error::too_many_requests,
    ];

    rocket = rocket.register(base_path.as_str(), all_catchers);

    rocket
}
