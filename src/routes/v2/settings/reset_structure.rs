use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::ResetStructureRequest;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;
use crate::session_dek::SessionDekStore;

#[post("/reset-structure", data = "<payload>")]
pub async fn reset_structure(
    pool: &State<PgPool>,
    store: &State<SessionDekStore>,
    user: CurrentUser,
    payload: Json<ResetStructureRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    // Look up the session DEK for the authenticated principal.
    let principal_id = user.principal_id().ok_or(AppError::Unauthorized)?;
    let dek = store.get_cloned(&principal_id).await;
    let dek = dek.ok_or(AppError::Unauthorized)?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    service.reset_structure(&user.id, &payload.password, &dek).await?;
    Ok(Status::NoContent)
}
