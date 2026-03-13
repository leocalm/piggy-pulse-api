use rocket::delete;
use rocket::http::Status;

use crate::auth::CurrentUser;

#[delete("/<_id>")]
pub async fn delete_category(_user: CurrentUser, _id: &str) -> Status {
    todo!()
}
