mod create;
mod delete;
mod get;
mod list;
mod schedule;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_periods,
        create::create_period,
        schedule::get_schedule,
        schedule::create_schedule,
        schedule::update_schedule,
        schedule::delete_schedule,
        get::get_period,
        update::update_period,
        delete::delete_period,
    ]
}
