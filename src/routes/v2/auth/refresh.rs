use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::RefreshResponse;
use crate::error::app_error::AppError;
use crate::service::auth::AuthService;

#[post("/refresh")]
pub async fn refresh(pool: &State<PgPool>, config: &State<Config>, user: CurrentUser) -> Result<Json<RefreshResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    auth.refresh_session(
        &user.id,
        user.session_id,
        None, // user_agent not needed for cookie-based refresh
        None, // client_ip not needed for cookie-based refresh
    )
    .await?;

    Ok(Json(RefreshResponse {
        token: "session_refreshed".to_string(),
    }))
}
