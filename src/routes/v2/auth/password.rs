use rocket::http::Status;
use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::ChangePasswordRequest;

#[put("/password", data = "<_payload>")]
pub async fn change_password(_user: CurrentUser, _payload: Json<ChangePasswordRequest>) -> Status {
    todo!()
}
