mod forgot_password;
pub(crate) mod login;
mod logout;
mod me;
mod password;
mod refresh;
mod register;
mod reset_password;
mod two_factor;
mod unlock;
mod wrapped_dek;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        login::login,
        register::register,
        logout::logout,
        me::me,
        refresh::refresh,
        password::change_password,
        forgot_password::forgot_password,
        reset_password::reset_password,
        unlock::unlock,
        wrapped_dek::get_wrapped_dek,
        wrapped_dek::update_wrapped_dek,
    ]
}

pub fn two_factor_routes() -> Vec<rocket::Route> {
    two_factor::routes()
}
