use rocket::get;
use rocket::serde::json::Json;

use crate::auth::CurrentUser;
use crate::dto::accounts::AccountOptionListResponse;

#[get("/options")]
pub async fn get_account_options(_user: CurrentUser) -> Json<AccountOptionListResponse> {
    todo!()
}
