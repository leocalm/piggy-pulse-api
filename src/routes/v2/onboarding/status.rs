use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::misc::OnboardingStatusResponse;

#[get("/status")]
pub async fn get_onboarding_status(_user: CurrentUser) -> Json<OnboardingStatusResponse> {
    todo!()
}
