mod auth;
mod config;
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
    //   RUST_LOG=budget=debug             - Set budget crate to debug
    //   RUST_LOG=budget::routes=trace     - Set specific module to trace
    //   RUST_LOG=info,budget::routes=debug - Global info, routes at debug
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).with_target(true).with_line_number(true);

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}

fn ensure_rocket_secret_key() {
    let profile = std::env::var("ROCKET_PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile != "debug" && std::env::var("ROCKET_SECRET_KEY").is_err() {
        panic!("ROCKET_SECRET_KEY must be set for non-debug profiles. Generate one with: openssl rand -base64 32");
    }
}

fn build_cors(cors_config: &config::CorsConfig) -> CorsOptions {
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
        let limiter = Arc::new(RateLimiter::new(rate_limit_config.clone()));
        limiter.clone().spawn_cleanup_task();

        Box::pin(async move { rocket.manage(limiter) })
    })
}

pub fn build_rocket(config: Config) -> Rocket<Build> {
    init_tracing(&config.logging.level, config.logging.json_format);
    ensure_rocket_secret_key();

    let cors = build_cors(&config.cors).to_cors().expect("Failed to create CORS fairing");

    let base_paths = collect_base_paths(&config.api);

    let mut rocket = rocket::build()
        .attach(stage_rate_limiter(config.rate_limit.clone()))
        .attach(cors)
        .attach(RequestLogger) // Attach request/response logging middleware
        .attach(stage_db(config.database));

    let (primary_base_path, additional_base_paths) = base_paths.split_first().expect("API base paths must include at least one entry");

    let settings = rocket_okapi::settings::OpenApiSettings::default();
    rocket_okapi::mount_endpoints_and_merged_docs! {
        rocket, primary_base_path.clone(), settings,
        "/accounts" => app_routes::account::routes(),
        "/users" => app_routes::user::routes(),
        "/currency" => app_routes::currency::routes(),
        "/categories" => app_routes::category::routes(),
        "/budgets" => app_routes::budget::routes(),
        "/budget-categories" => app_routes::budget_category::routes(),
        "/transactions" => app_routes::transaction::routes(),
        "/vendors" => app_routes::vendor::routes(),
        "/health" => app_routes::health::routes(),
        "/dashboard" => app_routes::dashboard::routes(),
        "/budget_period" => app_routes::budget_period::routes(),
    }

    let docs_path = join_base_path(primary_base_path, "docs");
    let primary_openapi_url = join_base_path(primary_base_path, "openapi.json");
    rocket = rocket.mount(docs_path, make_swagger_ui(&get_swagger_config(&primary_openapi_url)));

    rocket = rocket.register(
        primary_base_path.as_str(),
        catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
    );

    for base_path in additional_base_paths {
        let settings = rocket_okapi::settings::OpenApiSettings::default();
        rocket_okapi::mount_endpoints_and_merged_docs! {
            rocket, base_path.clone(), settings,
            "/accounts" => app_routes::account::routes(),
            "/users" => app_routes::user::routes(),
            "/currency" => app_routes::currency::routes(),
            "/categories" => app_routes::category::routes(),
            "/budgets" => app_routes::budget::routes(),
            "/budget-categories" => app_routes::budget_category::routes(),
            "/transactions" => app_routes::transaction::routes(),
            "/vendors" => app_routes::vendor::routes(),
            "/health" => app_routes::health::routes(),
            "/dashboard" => app_routes::dashboard::routes(),
            "/budget_period" => app_routes::budget_period::routes(),
        }

        let docs_path = join_base_path(base_path, "docs");
        let docs_openapi_url = join_base_path(base_path, "openapi.json");
        rocket = rocket.mount(docs_path, make_swagger_ui(&get_swagger_config(&docs_openapi_url)));

        rocket = rocket.register(
            base_path.as_str(),
            catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
        );
    }

    rocket
}

// TODO: allowance accounts
