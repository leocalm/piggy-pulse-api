use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[get("/data")]
pub async fn export_data(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<serde_json::Value>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let data = service.export_data(&user.id).await?;
    Ok(Json(data))
}
