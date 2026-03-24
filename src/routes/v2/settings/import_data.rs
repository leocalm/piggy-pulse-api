use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[post("/data", data = "<payload>")]
pub async fn import_data(pool: &State<PgPool>, user: CurrentUser, payload: Json<serde_json::Value>) -> Result<(Status, Json<serde_json::Value>), AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let result = service.import_data(&user.id, &payload).await?;
    Ok((Status::Ok, Json(result)))
}
