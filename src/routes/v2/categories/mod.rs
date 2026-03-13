mod archive;
mod create;
mod delete;
mod list;
mod options;
mod overview;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_categories,
        create::create_category,
        options::list_category_options,
        overview::category_overview,
        update::update_category,
        delete::delete_category,
        archive::archive_category,
        unarchive::unarchive_category,
    ]
}
