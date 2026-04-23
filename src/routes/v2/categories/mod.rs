mod archive;
mod create;
mod delete;
mod list;
mod options;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_categories,
        create::create_category,
        options::list_category_options,
        update::update_category,
        delete::delete_category,
        archive::archive_category,
        unarchive::unarchive_category,
    ]
}
