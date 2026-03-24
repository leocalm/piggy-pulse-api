mod cancel;
mod create;
mod delete;
mod get;
mod list;
mod upcoming;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_subscriptions,
        create::create_subscription,
        upcoming::get_upcoming,
        get::get_subscription,
        update::update_subscription,
        delete::delete_subscription,
        cancel::cancel_subscription,
    ]
}
