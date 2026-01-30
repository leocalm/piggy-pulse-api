use crate::auth::CurrentUser;
use crate::database::account::AccountRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::account::{AccountRequest, AccountResponse};
use crate::models::pagination::{PaginatedResponse, PaginationParams};
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

#[rocket::get("/?<page>&<limit>")]
pub async fn list_all_accounts(pool: &State<Pool>, _current_user: CurrentUser, page: Option<i64>, limit: Option<i64>) -> Result<Json<PaginatedResponse<AccountResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let account_service = AccountService::new(&repo);

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let responses = account_service.list_accounts(pagination.as_ref()).await?;
    let total = responses.len() as i64;

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
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
