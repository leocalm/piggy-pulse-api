use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::TwoFactorDisableRequest;

#[post("/disable", data = "<_payload>")]
pub async fn disable_two_factor(_user: CurrentUser, _payload: Json<TwoFactorDisableRequest>) -> Status {
    todo!()
}
