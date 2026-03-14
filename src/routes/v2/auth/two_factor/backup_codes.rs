use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{BackupCodesResponse, RegenerateBackupCodesRequest};
use crate::error::app_error::AppError;
use crate::service::two_factor::TwoFactorService;

#[post("/backup-codes/regenerate", data = "<payload>")]
pub async fn regenerate_backup_codes(
    pool: &State<PgPool>,
    config: &State<Config>,
    user: CurrentUser,
    payload: Json<RegenerateBackupCodesRequest>,
) -> Result<Json<BackupCodesResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tfa = TwoFactorService::new(&repo, config);

    let codes = tfa.regenerate_backup_codes(&user.id, &payload.code).await?;
    Ok(Json(BackupCodesResponse(codes)))
}
