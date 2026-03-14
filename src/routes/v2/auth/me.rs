use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::UserResponse;
use crate::error::app_error::AppError;
use crate::service::auth::AuthService;

#[get("/me")]
pub async fn me(pool: &State<PgPool>, config: &State<Config>, user: CurrentUser) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    let user_response = auth.get_user_response(&user.id).await?;
    Ok(Json(user_response))
}
