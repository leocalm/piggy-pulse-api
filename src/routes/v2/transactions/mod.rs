mod create;
mod delete;
mod list;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list::list_transactions,
        create::create_transaction,
        update::update_transaction,
        delete::delete_transaction,
    ]
}
