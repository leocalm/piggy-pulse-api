use rocket::delete;
use rocket::http::Status;

use crate::auth::CurrentUser;

#[delete("/<_id>")]
pub async fn revoke_session(_user: CurrentUser, _id: &str) -> Status {
    todo!()
}
