use rocket::serde::json::Json;
use rocket::{get, put};

use crate::auth::CurrentUser;
use crate::dto::settings::{ProfileResponse, UpdateProfileRequest};

#[get("/profile")]
pub async fn get_profile(_user: CurrentUser) -> Json<ProfileResponse> {
    todo!()
}

#[put("/profile", data = "<_payload>")]
pub async fn update_profile(_user: CurrentUser, _payload: Json<UpdateProfileRequest>) -> Json<ProfileResponse> {
    todo!()
}
