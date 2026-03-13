use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountBalanceHistoryResponse;

#[get("/<_id>/balance-history?<_period_id>")]
pub async fn get_balance_history(_user: CurrentUser, _id: &str, _period_id: Option<String>) -> Json<AccountBalanceHistoryResponse> {
    todo!()
}
