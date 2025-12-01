mod auth;
mod database;
mod db;
mod error;
mod models;
mod routes;

use crate::db::stage_db;
use crate::routes as app_routes;
use rocket::{catchers, routes, Build, Rocket};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_line_number(true)
        .init();
}

#[rocket::launch]
fn rocket() -> Rocket<Build> {
    dotenvy::dotenv().ok();
    init_tracing();

    rocket::build()
        .attach(stage_db())
        .mount(
            "/api",
            routes![
                app_routes::user::post_user,
                app_routes::user::post_user_login,
                app_routes::user::post_user_logout,
                app_routes::account::create_account,
                app_routes::account::list_all_accounts,
                app_routes::account::get_account,
                app_routes::account::delete_account,
                app_routes::currency::get_currency,
                app_routes::currency::create_currency,
                app_routes::currency::get_currencies,
                app_routes::currency::delete_currency,
                app_routes::budget::create_budget,
                app_routes::budget::list_all_budgets,
                app_routes::budget::get_budget,
                app_routes::budget::delete_budget,
                app_routes::budget::put_budget,
                app_routes::category::create_category,
                app_routes::category::list_all_categories,
                app_routes::category::get_category,
                app_routes::category::delete_category,
                app_routes::category::list_categories_not_in_budget,
                app_routes::budget_category::create_budget_category,
                app_routes::budget_category::list_all_budget_categories,
                app_routes::budget_category::get_budget_category,
                app_routes::budget_category::delete_budget_category,
                app_routes::transaction::create_transaction,
                app_routes::transaction::list_all_transactions,
                app_routes::transaction::get_transaction,
                app_routes::transaction::delete_transaction,
                app_routes::vendor::create_vendor,
                app_routes::vendor::list_all_vendors,
                app_routes::vendor::get_vendor,
                app_routes::vendor::delete_vendor,
                app_routes::health::healthcheck,
                app_routes::dashboard::get_balance_per_day,
                app_routes::dashboard::get_spent_per_category,
                app_routes::dashboard::get_monthly_burn_in,
            ],
        )
        .register(
            "/api",
            catchers![app_routes::error::not_found, app_routes::error::conflict],
        )
}

// TODO: Add routes to return information for the dashboard
// TODO: allowance accounts
