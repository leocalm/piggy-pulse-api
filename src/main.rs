use budget::{Config, build_rocket};
use rocket::{Build, Rocket};

#[rocket::launch]
fn rocket() -> Rocket<Build> {
    let config = Config::load().expect("Failed to load configuration");
    build_rocket(config)
}
