use rocket::http::Status;
use rocket::post;

use crate::auth::CurrentUser;

#[post("/<_id>/transactions/<_tx_id>/include")]
pub async fn include_overlay_transaction(_user: CurrentUser, _id: &str, _tx_id: &str) -> Status {
    todo!()
}
