use rocket::http::Status;
use rocket::post;

use crate::auth::CurrentUser;

#[post("/logout")]
pub async fn logout(_user: CurrentUser) -> Status {
    todo!()
}
