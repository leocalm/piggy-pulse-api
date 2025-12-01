use crate::auth::CurrentUser;
use crate::database::currency;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::currency::{CurrencyRequest, CurrencyResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use uuid::Uuid;

#[rocket::post("/currency", data = "<payload>")]
pub async fn create_currency(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    payload: Json<CurrencyRequest>,
) -> Result<(Status, Json<CurrencyResponse>), AppError> {
    let client = get_client(pool).await?;
    let currency = currency::create_currency(&client, &payload).await?;
    Ok((Status::Created, Json(CurrencyResponse::from(&currency))))
}

#[rocket::get("/currency/<code>")]
pub async fn get_currency(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    code: &str,
) -> Result<Json<CurrencyResponse>, AppError> {
    let client = get_client(pool).await?;
    if let Some(currency) = currency::get_currency_by_code(&client, code).await? {
        Ok(Json(CurrencyResponse::from(&currency)))
    } else {
        Err(AppError::NotFound(format!("Currency not found: {}", code)))
    }
}

#[rocket::get("/currency/name/<name>")]
pub async fn get_currencies(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    name: &str,
) -> Result<Json<Vec<CurrencyResponse>>, AppError> {
    let client = get_client(pool).await?;
    let currencies = currency::get_currencies(&client, name).await?;
    Ok(Json(
        currencies.iter().map(CurrencyResponse::from).collect(),
    ))
}

#[rocket::delete("/currency/<id>")]
pub async fn delete_currency(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    id: &str,
) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    currency::delete_currency(&client, &Uuid::parse_str(id)?).await?;
    Ok(Status::Ok)
}
