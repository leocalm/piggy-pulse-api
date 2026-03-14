use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountDetailsResponse;

#[get("/<_id>/details?<_period_id>")]
pub async fn get_account_details(_user: CurrentUser, _id: &str, _period_id: Option<String>) -> Json<AccountDetailsResponse> {
    todo!()
}
