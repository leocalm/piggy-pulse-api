use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::settings::SessionListResponse;

#[get("/")]
pub async fn list_sessions(_user: CurrentUser) -> Json<SessionListResponse> {
    todo!()
}
