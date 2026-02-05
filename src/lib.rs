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

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_line_number(true);

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
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

fn get_swagger_config() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "/api/openapi.json".to_owned(),
        ..Default::default()
    }
}

fn stage_rate_limiter(rate_limit_config: config::RateLimitConfig) -> AdHoc {
    AdHoc::on_ignite("Rate Limiter", move |rocket| {
        let limiter = Arc::new(RateLimiter::new(rate_limit_config.clone()));
        limiter.clone().spawn_cleanup_task();

        Box::pin(async move { rocket.manage(limiter) })
    })
}

pub fn build_rocket(config: Config) -> Rocket<Build> {
    dotenvy::dotenv().ok();
    init_tracing(&config.logging.level, config.logging.json_format);

    let cors = build_cors(&config.cors).to_cors().expect("Failed to create CORS fairing");

    let settings = rocket_okapi::settings::OpenApiSettings::default();

    let mut rocket = rocket::build()
        .attach(stage_rate_limiter(config.rate_limit.clone()))
        .attach(cors)
        .attach(RequestLogger) // Attach request/response logging middleware
        .attach(stage_db(config.database));

    rocket_okapi::mount_endpoints_and_merged_docs! {
        rocket, "/api".to_owned(), settings,
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

    rocket.mount("/api/docs", make_swagger_ui(&get_swagger_config())).register(
        "/api",
        catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
    )
}

// TODO: allowance accounts
