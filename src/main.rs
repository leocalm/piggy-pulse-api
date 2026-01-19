use budget::build_rocket;
use rocket::{Build, Rocket};

#[rocket::launch]
fn rocket() -> Rocket<Build> {
    build_rocket()
}
