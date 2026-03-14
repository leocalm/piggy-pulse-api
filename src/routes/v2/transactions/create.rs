use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::transactions::{CreateTransactionRequest, TransactionResponse};

#[post("/", data = "<_payload>")]
pub async fn create_transaction(_user: CurrentUser, _payload: Json<CreateTransactionRequest>) -> (Status, Json<TransactionResponse>) {
    todo!()
}
