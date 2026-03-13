use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountSummaryListResponse;

#[get("/summary?<_period_id>&<_cursor>&<_limit>")]
pub async fn list_account_summaries(
    _user: CurrentUser,
    _period_id: Option<String>,
    _cursor: Option<String>,
    _limit: Option<u32>,
) -> Json<AccountSummaryListResponse> {
    todo!()
}
