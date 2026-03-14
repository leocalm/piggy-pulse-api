use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::transactions::TransactionListResponse;

#[get("/?<_period_id>&<_cursor>&<_limit>&<_direction>&<_account_id>&<_category_id>&<_vendor_id>&<_from_date>&<_to_date>")]
#[allow(clippy::too_many_arguments)]
pub async fn list_transactions(
    _user: CurrentUser,
    _period_id: Option<String>,
    _cursor: Option<String>,
    _limit: Option<u32>,
    _direction: Option<String>,
    _account_id: Option<String>,
    _category_id: Option<String>,
    _vendor_id: Option<String>,
    _from_date: Option<String>,
    _to_date: Option<String>,
) -> Json<TransactionListResponse> {
    todo!()
}
