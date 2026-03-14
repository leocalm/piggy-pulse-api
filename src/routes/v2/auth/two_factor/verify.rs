use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::AuthenticatedResponse;
use crate::dto::auth::TwoFactorCompleteRequest;

#[post("/verify", data = "<_payload>")]
pub async fn verify_two_factor(_user: CurrentUser, _payload: Json<TwoFactorCompleteRequest>) -> Json<AuthenticatedResponse> {
    todo!()
}
