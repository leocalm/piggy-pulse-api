use rocket::http::Status;
use rocket::post;

use crate::auth::CurrentUser;

#[post("/complete")]
pub async fn complete_onboarding(_user: CurrentUser) -> Status {
    todo!()
}
