mod complete;
mod status;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![status::get_onboarding_status, complete::complete_onboarding]
}
