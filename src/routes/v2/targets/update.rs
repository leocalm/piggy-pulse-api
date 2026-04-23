use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{EncryptedTargetResponse, UpdateTargetRequest};
use crate::error::app_error::AppError;
use crate::service::category::CategoryService;

#[put("/<id>", data = "<payload>")]
pub async fn update_target(
    pool: &State<PgPool>,
    user: CurrentUser,
    dek: Dek,
    id: &str,
    payload: Json<UpdateTargetRequest>,
) -> Result<Json<EncryptedTargetResponse>, AppError> {
    payload.validate()?;
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid target id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CategoryService::new(&repo);
    Ok(Json(service.update_target(&uuid, &payload, &user.id, &dek).await?))
}
