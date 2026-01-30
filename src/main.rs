use budget::build_rocket;
use rocket::{Build, Rocket};

#[rocket::launch]
fn rocket() -> Rocket<Build> {
    let database_url = &std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    build_rocket(database_url.to_owned())
}
