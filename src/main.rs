use budget::{build_rocket, Config};
use rocket::{Build, Rocket};

#[rocket::launch]
fn rocket() -> Rocket<Build> {
    // Load local .env overrides for development convenience.
    let _ = dotenvy::dotenv();
    let config = Config::load().expect("Failed to load configuration");
    build_rocket(config)
}
