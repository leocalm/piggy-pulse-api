mod archive;
mod create;
mod delete;
mod detail;
mod list;
mod merge;
mod options;
mod stats;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        stats::get_vendor_stats,
        list::list_vendors,
        create::create_vendor,
        options::list_vendor_options,
        detail::get_vendor_detail,
        merge::merge_vendor,
        update::update_vendor,
        delete::delete_vendor,
        archive::archive_vendor,
        unarchive::unarchive_vendor,
    ]
}
