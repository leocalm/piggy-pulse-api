use rocket::serde::Serialize;
use rocket::serde::json::Json;
use rocket::{Request, catch};

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Error {
    pub message: String,
}

// no DB changes required in error routes

#[catch(404)]
pub fn not_found(_: &Request) -> Json<Error> {
    Json(Error {
        message: "Not found".to_string(),
    })
}

#[catch(409)]
pub fn conflict(_: &Request) -> Json<Error> {
    Json(Error {
        message: "Conflict".to_string(),
    })
}
