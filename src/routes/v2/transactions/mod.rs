mod batch;
mod create;
mod delete;
mod update;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        create::create_transaction,
        batch::batch_create_transactions,
        update::update_transaction,
        delete::delete_transaction,
    ]
}
