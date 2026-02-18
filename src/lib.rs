mod auth;
mod compatibility_adapter;
mod config;
mod cron_tasks;
mod database;
mod db;
mod error;
mod middleware;
mod models;
mod routes;
mod service;

#[cfg(test)]
pub mod test_utils;

pub use config::Config;
pub use cron_tasks::{GeneratePeriodsResult, generate_periods};

use crate::db::stage_db;
use crate::middleware::RequestLogger;
use crate::middleware::rate_limit::RateLimiter;
use crate::routes as app_routes;
use rocket::fairing::AdHoc;
use rocket::{Build, Rocket, catchers, http::Method};
use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_okapi::swagger_ui::{SwaggerUIConfig, make_swagger_ui};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn init_tracing(log_level: &str, json_format: bool) {
    // Configure logging with environment variable support
    // RUST_LOG environment variable can be used for fine-grained control per module:
    // Examples:
    //   RUST_LOG=debug                    - Set all to debug
    //   RUST_LOG=piggy_pulse=debug             - Set piggy_pulse crate to debug
    //   RUST_LOG=piggy_pulse::routes=trace     - Set specific module to trace
    //   RUST_LOG=info,piggy_pulse::routes=debug - Global info, routes at debug
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).with_target(true).with_line_number(true);

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
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

fn get_swagger_config(openapi_url: &str) -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: openapi_url.to_string(),
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

fn collect_base_paths(api_config: &config::ApiConfig) -> Vec<String> {
    let mut normalized: Vec<String> = Vec::new();
    let mut push_unique = |path: String| {
        if !normalized.contains(&path) {
            normalized.push(path);
        }
    };

    push_unique(normalize_base_path(&api_config.base_path));

    for extra in &api_config.additional_base_paths {
        let normalized_extra = normalize_base_path(extra);
        if !normalized_extra.is_empty() {
            push_unique(normalized_extra);
        }
    }

    normalized
}

fn stage_rate_limiter(rate_limit_config: config::RateLimitConfig) -> AdHoc {
    AdHoc::on_ignite("Rate Limiter", move |rocket| {
        Box::pin(async move {
            let limiter = match RateLimiter::new(rate_limit_config.clone()).await {
                Ok(limiter) => Arc::new(limiter),
                Err(err) => {
                    eprintln!("Failed to initialize rate limiter: {}", err);
                    std::process::exit(1);
                }
            };

            limiter.clone().spawn_cleanup_task();

            rocket.manage(limiter)
        })
    })
}

fn mount_api_routes(mut rocket: Rocket<Build>, base_path: &str) -> Rocket<Build> {
    rocket = rocket.mount(join_base_path(base_path, "accounts"), app_routes::account::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "users"), app_routes::user::routes().0);
    rocket = rocket.mount(join_base_path(base_path, ""), app_routes::password_reset::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "settings"), app_routes::settings::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "currency"), app_routes::currency::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "categories"), app_routes::category::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "budget-categories"), app_routes::budget_category::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "transactions"), app_routes::transaction::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "vendors"), app_routes::vendor::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "health"), app_routes::health::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "dashboard"), app_routes::dashboard::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "budget_period"), app_routes::budget_period::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "overlays"), app_routes::overlay::routes().0);
    rocket = rocket.mount(join_base_path(base_path, "two-factor"), app_routes::two_factor::routes().0);
    rocket
}

pub fn build_rocket(config: Config) -> Rocket<Build> {
    dotenvy::dotenv().ok();
    init_tracing(&config.logging.level, config.logging.json_format);
    ensure_rocket_secret_key();
    ensure_two_factor_encryption_key(&config.two_factor);

    let cors = build_cors(&config.cors).to_cors().expect("Failed to create CORS fairing");

    let base_paths = collect_base_paths(&config.api);

    let mut rocket = rocket::build()
        .manage(config.clone())
        .attach(stage_rate_limiter(config.rate_limit.clone()))
        .attach(cors)
        .attach(RequestLogger) // Attach request/response logging middleware
        .attach(stage_db(config.database));

    let (primary_base_path, additional_base_paths) = base_paths.split_first().expect("API base paths must include at least one entry");

    if config.api.expose_docs {
        let settings = rocket_okapi::settings::OpenApiSettings::default();
        rocket_okapi::mount_endpoints_and_merged_docs! {
            rocket, primary_base_path.clone(), settings,
            "/accounts" => app_routes::account::routes(),
            "/users" => app_routes::user::routes(),
            "" => app_routes::password_reset::routes(),
            "/settings" => app_routes::settings::routes(),
            "/currency" => app_routes::currency::routes(),
            "/categories" => app_routes::category::routes(),
            "/budget-categories" => app_routes::budget_category::routes(),
            "/transactions" => app_routes::transaction::routes(),
            "/vendors" => app_routes::vendor::routes(),
            "/health" => app_routes::health::routes(),
            "/dashboard" => app_routes::dashboard::routes(),
            "/budget_period" => app_routes::budget_period::routes(),
            "/overlays" => app_routes::overlay::routes(),
            "/two-factor" => app_routes::two_factor::routes(),
        }
        if config.api.expose_swagger_ui {
            let docs_path = join_base_path(primary_base_path, "docs");
            let primary_openapi_url = join_base_path(primary_base_path, "openapi.json");
            rocket = rocket.mount(docs_path, make_swagger_ui(&get_swagger_config(&primary_openapi_url)));
        }
    } else {
        rocket = mount_api_routes(rocket, primary_base_path);
    }

    rocket = rocket.register(
        primary_base_path.as_str(),
        catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
    );

    for base_path in additional_base_paths {
        if config.api.expose_docs {
            let settings = rocket_okapi::settings::OpenApiSettings::default();
            rocket_okapi::mount_endpoints_and_merged_docs! {
                rocket, base_path.clone(), settings,
                "/accounts" => app_routes::account::routes(),
                "/users" => app_routes::user::routes(),
                "" => app_routes::password_reset::routes(),
                "/settings" => app_routes::settings::routes(),
                "/currency" => app_routes::currency::routes(),
                "/categories" => app_routes::category::routes(),
                "/budget-categories" => app_routes::budget_category::routes(),
                "/transactions" => app_routes::transaction::routes(),
                "/vendors" => app_routes::vendor::routes(),
                "/health" => app_routes::health::routes(),
                "/dashboard" => app_routes::dashboard::routes(),
                "/budget_period" => app_routes::budget_period::routes(),
                "/overlays" => app_routes::overlay::routes(),
                "/two-factor" => app_routes::two_factor::routes(),
            }
            if config.api.expose_swagger_ui {
                let docs_path = join_base_path(base_path, "docs");
                let docs_openapi_url = join_base_path(base_path, "openapi.json");
                rocket = rocket.mount(docs_path, make_swagger_ui(&get_swagger_config(&docs_openapi_url)));
            }
        } else {
            rocket = mount_api_routes(rocket, base_path);
        }

        rocket = rocket.register(
            base_path.as_str(),
            catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
        );
    }

    rocket
}
