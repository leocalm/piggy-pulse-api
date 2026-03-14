use rocket::put;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::transactions::{TransactionResponse, UpdateTransactionRequest};

#[put("/<_id>", data = "<_payload>")]
pub async fn update_transaction(_user: CurrentUser, _id: &str, _payload: Json<UpdateTransactionRequest>) -> Json<TransactionResponse> {
    todo!()
}
