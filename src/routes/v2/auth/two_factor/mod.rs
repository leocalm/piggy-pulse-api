mod backup_codes;
mod disable;
mod emergency_disable_confirm;
mod emergency_disable_request;
mod enable;
mod status;
mod verify;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        status::two_factor_status,
        enable::enable_two_factor,
        verify::verify_two_factor,
        disable::disable_two_factor,
        backup_codes::regenerate_backup_codes,
        emergency_disable_request::emergency_disable_request,
        emergency_disable_confirm::emergency_disable_confirm,
    ]
}
