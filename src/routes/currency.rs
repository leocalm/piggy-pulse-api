use crate::auth::CurrentUser;
use crate::database::currency::CurrencyRepository;
use crate::database::postgres_repository::PostgresRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::currency::{CurrencyRequest, CurrencyResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_currency(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<CurrencyRequest>,
) -> Result<(Status, Json<CurrencyResponse>), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let currency = repo.create_currency(&payload).await?;
    Ok((Status::Created, Json(CurrencyResponse::from(&currency))))
}

#[rocket::get("/<code>")]
pub async fn get_currency(pool: &State<Pool>, _current_user: CurrentUser, code: &str) -> Result<Json<CurrencyResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    if let Some(currency) = repo.get_currency_by_code(code).await? {
        Ok(Json(CurrencyResponse::from(&currency)))
    } else {
        Err(AppError::NotFound(format!("Currency not found: {}", code)))
    }
}

#[rocket::get("/name/<name>")]
pub async fn get_currencies(pool: &State<Pool>, _current_user: CurrentUser, name: &str) -> Result<Json<Vec<CurrencyResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let currencies = repo.get_currencies(name).await?;
    Ok(Json(currencies.iter().map(CurrencyResponse::from).collect()))
}

#[rocket::delete("/<id>")]
pub async fn delete_currency(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid currency id", e))?;
    repo.delete_currency(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_currency(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<CurrencyRequest>,
) -> Result<Json<CurrencyResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid currency id", e))?;
    let currency = repo.update_currency(&uuid, &payload).await?;
    Ok(Json(CurrencyResponse::from(&currency)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_currency, get_currency, get_currencies, delete_currency, put_currency]
}
