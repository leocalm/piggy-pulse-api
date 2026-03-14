mod create;
mod exclude;
mod list;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![list::list_targets, create::create_target, update::update_target, exclude::exclude_target,]
}
