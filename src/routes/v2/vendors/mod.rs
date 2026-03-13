mod archive;
mod create;
mod delete;
mod list;
mod options;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_vendors,
        create::create_vendor,
        options::list_vendor_options,
        update::update_vendor,
        delete::delete_vendor,
        archive::archive_vendor,
        unarchive::unarchive_vendor,
    ]
}
