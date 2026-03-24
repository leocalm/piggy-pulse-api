mod archive;
mod create;
mod delete;
mod detail;
mod list;
mod options;
mod overview;
mod trend;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_categories,
        create::create_category,
        options::list_category_options,
        overview::category_overview,
        detail::get_category_detail,
        trend::get_category_trend,
        update::update_category,
        delete::delete_category,
        archive::archive_category,
        unarchive::unarchive_category,
    ]
}
