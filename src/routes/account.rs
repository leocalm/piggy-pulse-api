use crate::auth::CurrentUser;
use crate::database::account::AccountRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::account::{AccountRequest, AccountResponse};
use crate::service::account::AccountService;
use deadpool_postgres::Pool;
use rocket::serde::json::Json;
use rocket::{http::Status, routes, State};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_account(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<AccountRequest>,
) -> Result<(Status, Json<AccountResponse>), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let account = repo.create_account(&payload).await?;
    Ok((Status::Created, Json(AccountResponse::from(&account))))
}

#[rocket::get("/")]
pub async fn list_all_accounts(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let account_service = AccountService::new(&repo);

    Ok(Json(account_service.list_accounts().await?))
}

#[rocket::get("/<id>")]
pub async fn get_account(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<AccountResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(account) = repo.get_account_by_id(&uuid).await? {
        Ok(Json(AccountResponse::from(&account)))
    } else {
        Err(AppError::NotFound("Account not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_account(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_account(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_account(pool: &State<Pool>, _current_user: CurrentUser, id: &str, payload: Json<AccountRequest>) -> Result<Json<AccountResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let account = repo.update_account(&uuid, &payload).await?;
    Ok(Json(AccountResponse::from(&account)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_account, list_all_accounts, get_account, delete_account, put_account]
}
