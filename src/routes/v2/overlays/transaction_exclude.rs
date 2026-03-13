use rocket::delete;
use rocket::http::Status;

use crate::auth::CurrentUser;

#[delete("/<_id>/transactions/<_tx_id>/exclude")]
pub async fn exclude_overlay_transaction(_user: CurrentUser, _id: &str, _tx_id: &str) -> Status {
    todo!()
}
