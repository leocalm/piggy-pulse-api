use rocket::get;
use rocket::serde::json::Json;

use crate::dto::misc::UnlockResponse;

#[get("/?<_token>")]
pub async fn unlock(_token: String) -> Json<UnlockResponse> {
    todo!()
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![unlock]
}
