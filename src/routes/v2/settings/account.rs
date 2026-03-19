use rocket::State;
use rocket::delete;
use rocket::http::{Cookie, Status};
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::DeleteAccountRequest;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[delete("/account", data = "<payload>")]
pub async fn delete_account(
    pool: &State<PgPool>,
    user: CurrentUser,
    cookies: &rocket::http::CookieJar<'_>,
    payload: Json<DeleteAccountRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    service.delete_account(&user.id, &payload.password).await?;
    cookies.remove_private(Cookie::build("user").build());
    Ok(Status::NoContent)
}
