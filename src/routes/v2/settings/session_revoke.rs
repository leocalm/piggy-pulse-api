use rocket::State;
use rocket::delete;
use rocket::http::{Cookie, Status};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;
use crate::session_dek::SessionDekStore;

#[delete("/<id>")]
pub async fn revoke_session(
    pool: &State<PgPool>,
    user: CurrentUser,
    cookies: &rocket::http::CookieJar<'_>,
    dek_store: &State<SessionDekStore>,
    id: &str,
) -> Result<Status, AppError> {
    let session_id = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid session id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    service.revoke_session(&session_id, &user.id).await?;

    dek_store.remove(&session_id).await;

    // Clear the cookie if the revoked session is the current one
    if user.session_id == Some(session_id) {
        cookies.remove_private(Cookie::build("user").build());
    }

    Ok(Status::NoContent)
}
