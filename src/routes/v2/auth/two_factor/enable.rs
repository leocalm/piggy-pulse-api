use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::TwoFactorEnableResponse;
use crate::error::app_error::AppError;
use crate::service::two_factor::TwoFactorService;

#[post("/enable")]
pub async fn enable_two_factor(pool: &State<PgPool>, config: &State<Config>, user: CurrentUser) -> Result<Json<TwoFactorEnableResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let response = tfa.enable(&user.id, &user.username).await?;
    Ok(Json(response))
}
