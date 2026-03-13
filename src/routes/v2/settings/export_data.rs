use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;

#[get("/data")]
pub async fn export_data(_user: CurrentUser) -> Json<serde_json::Value> {
    todo!()
}
