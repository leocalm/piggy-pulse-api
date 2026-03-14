mod create;
mod delete;
mod get;
mod list;
mod transaction_exclude;
mod transaction_include;
mod transactions;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_overlays,
        create::create_overlay,
        get::get_overlay,
        update::update_overlay,
        delete::delete_overlay,
        transactions::list_overlay_transactions,
        transaction_include::include_overlay_transaction,
        transaction_exclude::exclude_overlay_transaction,
    ]
}
