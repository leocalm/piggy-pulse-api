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
use rocket::{Build, Rocket, catchers};
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

pub fn build_rocket(config: Config) -> Rocket<Build> {
    init_tracing(&config.logging.level, config.logging.json_format);

    rocket::build()
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
        .register("/api", catchers![app_routes::error::not_found, app_routes::error::conflict])
}

// TODO: allowance accounts
