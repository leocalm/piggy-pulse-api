mod adjust_balance;
mod archive;
mod create;
mod delete;
mod get;
mod list;
mod options;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_accounts,
        options::get_account_options,
        create::create_account,
        get::get_account,
        update::update_account,
        delete::delete_account,
        archive::archive_account,
        unarchive::unarchive_account,
        adjust_balance::adjust_balance,
    ]
}
