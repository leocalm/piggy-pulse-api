use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::{AccountResponse, AdjustBalanceRequest};

#[post("/<_id>/adjust-balance", data = "<_payload>")]
pub async fn adjust_balance(_user: CurrentUser, _id: &str, _payload: Json<AdjustBalanceRequest>) -> Json<AccountResponse> {
    todo!()
}
