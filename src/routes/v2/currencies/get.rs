use rocket::get;
use rocket::serde::json::Json;

use crate::dto::misc::CurrencyResponse;

#[get("/<_code>")]
pub async fn get_currency(_code: &str) -> Json<CurrencyResponse> {
    todo!()
}
