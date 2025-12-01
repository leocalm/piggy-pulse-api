use crate::auth::CurrentUser;
use crate::database;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::account::{AccountRequest, AccountResponse};
use deadpool_postgres::Pool;
use rocket::serde::json::Json;
use rocket::{http::Status, State};
use uuid::Uuid;

#[rocket::post("/accounts", data = "<payload>")]
pub async fn create_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<AccountRequest>,
) -> Result<(Status, Json<AccountResponse>), AppError> {
    let client = get_client(pool).await?;
    let account = database::account::create_account(&client, &payload).await?;
    Ok((Status::Created, Json(AccountResponse::from(&account))))
}

#[rocket::get("/accounts")]
pub async fn list_all_accounts(
    pool: &State<Pool>,
    _current_user: CurrentUser,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let client = get_client(pool).await?;
    let accounts = database::account::list_accounts(&client).await?;
    Ok(Json(accounts.iter().map(AccountResponse::from).collect()))
}

#[rocket::get("/accounts/<id>")]
pub async fn get_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
) -> Result<Json<AccountResponse>, AppError> {
    let client = get_client(pool).await?;
    let uuid = Uuid::parse_str(id)?;
    if let Some(account) = database::account::get_account_by_id(&client, &uuid).await? {
        Ok(Json(AccountResponse::from(&account)))
    } else {
        Err(AppError::NotFound("Account not found".to_string()))
    }
}

#[rocket::delete("/accounts/<id>")]
pub async fn delete_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let uuid = Uuid::parse_str(id)?;
    database::account::delete_account(&client, &uuid).await?;
    Ok(Status::Ok)
}
