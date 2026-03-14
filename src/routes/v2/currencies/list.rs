use rocket::get;
use rocket::serde::json::Json;

use crate::dto::misc::CurrencyListResponse;

#[get("/")]
pub async fn list_currencies() -> Json<CurrencyListResponse> {
    todo!()
}
