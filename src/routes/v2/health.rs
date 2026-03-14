use rocket::get;
use rocket::serde::json::Json;

use crate::dto::health::HealthResponse;

#[get("/")]
pub async fn health_check() -> Json<HealthResponse> {
    todo!()
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![health_check]
}
