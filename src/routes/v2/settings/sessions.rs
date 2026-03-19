use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::SessionListResponse;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

#[get("/")]
pub async fn list_sessions(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<SessionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let sessions = service.list_sessions(&user.id, user.session_id).await?;
    Ok(Json(sessions))
}
