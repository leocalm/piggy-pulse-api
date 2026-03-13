use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::overlay::OverlayTransactionListResponse;

#[get("/<_id>/transactions")]
pub async fn list_overlay_transactions(_user: CurrentUser, _id: &str) -> Json<OverlayTransactionListResponse> {
    todo!()
}
