use rocket::serde::json::Json;
use rocket::{get, put};

use crate::auth::CurrentUser;
use crate::dto::settings::{PreferencesResponse, UpdatePreferencesRequest};

#[get("/preferences")]
pub async fn get_preferences(_user: CurrentUser) -> Json<PreferencesResponse> {
    todo!()
}

#[put("/preferences", data = "<_payload>")]
pub async fn update_preferences(_user: CurrentUser, _payload: Json<UpdatePreferencesRequest>) -> Json<PreferencesResponse> {
    todo!()
}
