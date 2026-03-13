use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::dashboard::BudgetStabilityResponse;

#[get("/budget-stability?<_period_id>")]
pub async fn get_budget_stability(_user: CurrentUser, _period_id: Option<String>) -> Json<BudgetStabilityResponse> {
    todo!()
}
