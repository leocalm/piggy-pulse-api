use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::dashboard::NetPositionResponse;

#[get("/net-position?<_period_id>")]
pub async fn get_net_position(_user: CurrentUser, _period_id: Option<String>) -> Json<NetPositionResponse> {
    todo!()
}
