mod auth;
mod config;
mod database;
mod db;
mod error;
mod models;
mod routes;
mod service;

#[cfg(test)]
pub mod test_utils;

pub use config::Config;

use crate::db::stage_db;
use crate::routes as app_routes;
use rocket::{Build, Rocket, catchers, http::Method};
use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_okapi::swagger_ui::{SwaggerUIConfig, make_swagger_ui};
use tracing_subscriber::EnvFilter;

fn init_tracing(log_level: &str, json_format: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter).with_target(true).with_line_number(true);

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

pub fn build_rocket(config: Config) -> Rocket<Build> {
    init_tracing(&config.logging.level, config.logging.json_format);

    let cors = build_cors(&config.cors).to_cors().expect("Failed to create CORS fairing");

    rocket::build()
        .attach(cors)
        .attach(stage_db(config.database))
        .mount("/api/accounts", app_routes::account::routes())
        .mount("/api/users", app_routes::user::routes())
        .mount("/api/currency", app_routes::currency::routes())
        .mount("/api/categories", app_routes::category::routes())
        .mount("/api/budgets", app_routes::budget::routes())
        .mount("/api/budget-categories", app_routes::budget_category::routes())
        .mount("/api/transactions", app_routes::transaction::routes())
        .mount("/api/vendors", app_routes::vendor::routes())
        .mount("/api/health", app_routes::health::routes())
        .mount("/api/dashboard", app_routes::dashboard::routes())
        .mount("/api/budget_period", app_routes::budget_period::routes())
        .mount("/api/docs", make_swagger_ui(&get_swagger_config()))
        .register("/api", catchers![app_routes::error::not_found, app_routes::error::conflict])
}

// TODO: allowance accounts
