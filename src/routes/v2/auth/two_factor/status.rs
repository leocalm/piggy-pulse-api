use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::TwoFactorStatusResponse;
use crate::error::app_error::AppError;
use crate::service::two_factor::TwoFactorService;

#[get("/status")]
pub async fn two_factor_status(pool: &State<PgPool>, config: &State<Config>, user: CurrentUser) -> Result<Json<TwoFactorStatusResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let status = tfa.get_status(&user.id).await?;
    Ok(Json(status))
}
