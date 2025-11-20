use crate::auth::CurrentUser;
use crate::database::account::{create_account, get_account_by_id, list_accounts, update_account};
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::account::{AccountRequest, AccountResponse};
use deadpool_postgres::Pool;
use rocket::serde::json::Json;
use rocket::{State, http::Status};
use uuid::Uuid;

#[rocket::post("/accounts", data = "<payload>")]
pub async fn post_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<AccountRequest>,
) -> Result<(Status, Json<AccountResponse>), AppError> {
    let client = get_client(pool).await?;
    let account = create_account(&client, &payload).await?;

    if let Some(account) = account {
        let response = AccountResponse {
            id: account.id,
            name: account.name,
            color: account.color,
            icon: account.icon,
            account_type: account.account_type,
            currency: account.currency.currency,
            balance: account.balance,
        };
        Ok((Status::Created, Json(response)))
    } else {
        Err(AppError::Db("Error creating account".to_string()))
    }
}

#[rocket::get("/accounts")]
pub async fn list_all_accounts(
    pool: &State<Pool>,
    _current_user: CurrentUser,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let client = get_client(pool).await?;
    let accounts = list_accounts(&client).await?;

    let responses = accounts
        .into_iter()
        .map(|account| AccountResponse {
            id: account.id,
            name: account.name,
            color: account.color,
            icon: account.icon,
            account_type: account.account_type,
            currency: account.currency.currency,
            balance: account.balance,
        })
        .collect();

    Ok(Json(responses))
}

#[rocket::get("/accounts/<id>")]
pub async fn get_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
) -> Result<Json<AccountResponse>, AppError> {
    let client = get_client(pool).await?;
    let uuid =
        Uuid::parse_str(id).map_err(|_| AppError::BadRequest("Invalid account id".to_string()))?;

    if let Some(account) = get_account_by_id(&client, &uuid).await? {
        let response = AccountResponse {
            id: account.id,
            name: account.name,
            color: account.color,
            icon: account.icon,
            account_type: account.account_type,
            currency: account.currency.currency,
            balance: account.balance,
        };
        Ok(Json(response))
    } else {
        Err(AppError::NotFound("Account not found".to_string()))
    }
}

#[rocket::put("/accounts/<id>", data = "<payload>")]
pub async fn put_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<AccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let client = get_client(pool).await?;
    let uuid =
        Uuid::parse_str(id).map_err(|_| AppError::BadRequest("Invalid account id".to_string()))?;

    let account = update_account(&client, &uuid, &payload).await?;

    if let Some(account) = account {
        let response = AccountResponse {
            id: account.id,
            name: account.name,
            color: account.color,
            icon: account.icon,
            account_type: account.account_type,
            currency: account.currency.currency,
            balance: account.balance,
        };
        Ok(Json(response))
    } else {
        Err(AppError::NotFound("Account not found".to_string()))
    }
}
