use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::auth::TwoFactorStatusResponse;

#[get("/status")]
pub async fn two_factor_status(_user: CurrentUser) -> Json<TwoFactorStatusResponse> {
    todo!()
}
