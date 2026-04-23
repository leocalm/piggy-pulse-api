mod account;
mod preferences;
mod profile;
mod reset_structure;
mod session_revoke;
mod sessions;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        profile::get_profile,
        profile::update_profile,
        account::delete_account,
        preferences::get_preferences,
        preferences::update_preferences,
        reset_structure::reset_structure,
    ]
}

pub fn session_routes() -> Vec<rocket::Route> {
    rocket::routes![sessions::list_sessions, session_revoke::revoke_session]
}
