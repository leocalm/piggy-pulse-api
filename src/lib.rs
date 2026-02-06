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
use rocket_okapi::{get_openapi_route, okapi::merge::marge_spec_list};
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

    // Only enforce ROCKET_SECRET_KEY requirement for non-debug profiles
    if profile != "debug" && std::env::var("ROCKET_SECRET_KEY").is_err() {
        panic!(
            "ROCKET_SECRET_KEY is required for profile '{}'. Generate one with: openssl rand -base64 32",
            profile
        );
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

    let allowed_origins = if cors_config.allowed_origins.is_empty() {
        AllowedOrigins::some_exact::<&str>(&[])
    } else if is_wildcard {
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

struct RouteSpec {
    path: &'static str,
    routes: Vec<rocket::Route>,
    openapi: rocket_okapi::okapi::openapi3::OpenApi,
}

fn collect_route_specs() -> Vec<RouteSpec> {
    let (account_routes, account_openapi) = app_routes::account::routes();
    let (user_routes, user_openapi) = app_routes::user::routes();
    let (currency_routes, currency_openapi) = app_routes::currency::routes();
    let (category_routes, category_openapi) = app_routes::category::routes();
    let (budget_routes, budget_openapi) = app_routes::budget::routes();
    let (budget_category_routes, budget_category_openapi) = app_routes::budget_category::routes();
    let (transaction_routes, transaction_openapi) = app_routes::transaction::routes();
    let (vendor_routes, vendor_openapi) = app_routes::vendor::routes();
    let (health_routes, health_openapi) = app_routes::health::routes();
    let (dashboard_routes, dashboard_openapi) = app_routes::dashboard::routes();
    let (budget_period_routes, budget_period_openapi) = app_routes::budget_period::routes();

    vec![
        RouteSpec {
            path: "/accounts",
            routes: account_routes,
            openapi: account_openapi,
        },
        RouteSpec {
            path: "/users",
            routes: user_routes,
            openapi: user_openapi,
        },
        RouteSpec {
            path: "/currency",
            routes: currency_routes,
            openapi: currency_openapi,
        },
        RouteSpec {
            path: "/categories",
            routes: category_routes,
            openapi: category_openapi,
        },
        RouteSpec {
            path: "/budgets",
            routes: budget_routes,
            openapi: budget_openapi,
        },
        RouteSpec {
            path: "/budget-categories",
            routes: budget_category_routes,
            openapi: budget_category_openapi,
        },
        RouteSpec {
            path: "/transactions",
            routes: transaction_routes,
            openapi: transaction_openapi,
        },
        RouteSpec {
            path: "/vendors",
            routes: vendor_routes,
            openapi: vendor_openapi,
        },
        RouteSpec {
            path: "/health",
            routes: health_routes,
            openapi: health_openapi,
        },
        RouteSpec {
            path: "/dashboard",
            routes: dashboard_routes,
            openapi: dashboard_openapi,
        },
        RouteSpec {
            path: "/budget_period",
            routes: budget_period_routes,
            openapi: budget_period_openapi,
        },
    ]
}

fn mount_api_routes(mut rocket: Rocket<Build>, base_path: &str, enable_swagger: bool) -> Rocket<Build> {
    let route_specs = collect_route_specs();

    if enable_swagger {
        let mut openapi_list = Vec::new();
        for spec in route_specs {
            rocket = rocket.mount(format!("{}{}", base_path, spec.path), spec.routes);
            openapi_list.push((spec.path, spec.openapi));
        }

        let openapi_docs = match marge_spec_list(&openapi_list) {
            Ok(docs) => docs,
            Err(err) => panic!("Could not merge OpenAPI spec: {}", err),
        };

        let settings = rocket_okapi::settings::OpenApiSettings::default();
        rocket = rocket.mount(base_path, vec![get_openapi_route(openapi_docs, &settings)]);

        let docs_path = join_base_path(base_path, "docs");
        let openapi_url = join_base_path(base_path, "openapi.json");
        rocket = rocket.mount(docs_path, make_swagger_ui(&get_swagger_config(&openapi_url)));
    } else {
        for spec in route_specs {
            rocket = rocket.mount(format!("{}{}", base_path, spec.path), spec.routes);
        }
    }

    rocket
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
    let enable_swagger = config.api.enable_swagger;
    rocket = mount_api_routes(rocket, primary_base_path, enable_swagger);

    rocket = rocket.register(
        primary_base_path.as_str(),
        catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
    );

    for base_path in additional_base_paths {
        rocket = mount_api_routes(rocket, base_path, enable_swagger);

        rocket = rocket.register(
            base_path.as_str(),
            catchers![app_routes::error::not_found, app_routes::error::conflict, app_routes::error::too_many_requests],
        );
    }

    rocket
}

// TODO: allowance accounts
