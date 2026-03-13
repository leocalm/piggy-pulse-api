mod adjust_balance;
mod archive;
mod balance_history;
mod create;
mod delete;
mod details;
mod get;
mod list;
mod options;
mod summary;
mod unarchive;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_accounts,
        options::get_account_options,
        summary::list_account_summaries,
        create::create_account,
        get::get_account,
        update::update_account,
        delete::delete_account,
        archive::archive_account,
        unarchive::unarchive_account,
        adjust_balance::adjust_balance,
        balance_history::get_balance_history,
        details::get_account_details,
    ]
}
