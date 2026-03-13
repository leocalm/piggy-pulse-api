use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::TwoFactorEnableResponse;

#[post("/enable")]
pub async fn enable_two_factor(_user: CurrentUser) -> Json<TwoFactorEnableResponse> {
    todo!()
}
