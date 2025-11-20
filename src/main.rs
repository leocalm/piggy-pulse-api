mod auth;
mod database;
mod db;
mod error;
mod models;
mod routes;

use crate::db::stage_db;
use crate::routes::account::{get_account, list_all_accounts, post_account, put_account};
use crate::routes::budget::{health, list_budgets, post_budget};
use crate::routes::error::{conflict, not_found};
use crate::routes::user::{post_user, post_user_login, post_user_logout};
use rocket::{Build, Rocket, catchers, routes};
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
                health,
                post_budget,
                list_budgets,
                post_user,
                post_user_login,
                post_user_logout,
                post_account,
                list_all_accounts,
                get_account,
                put_account
            ],
        )
        .register("/api", catchers![not_found, conflict])
}
