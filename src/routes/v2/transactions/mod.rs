mod batch;
mod create;
mod delete;
mod list;
mod stats;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_transactions,
        stats::get_transaction_stats,
        create::create_transaction,
        batch::batch_create_transactions,
        update::update_transaction,
        delete::delete_transaction,
    ]
}
