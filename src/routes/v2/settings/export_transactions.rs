use rocket::get;

use crate::auth::CurrentUser;

#[get("/transactions")]
pub async fn export_transactions(_user: CurrentUser) -> String {
    todo!()
}
