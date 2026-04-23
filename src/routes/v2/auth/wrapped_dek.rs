use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use rocket::State;
use rocket::get;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::auth::{UpdateWrappedDekRequest, WrappedDekResponse};
use crate::error::app_error::AppError;

#[get("/wrapped-dek")]
pub async fn get_wrapped_dek(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<WrappedDekResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let (wrapped_dek, dek_wrap_params) = repo.get_wrapped_dek(&user.id).await?;
    Ok(Json(WrappedDekResponse {
        wrapped_dek: wrapped_dek.map(|b| B64.encode(&b)),
        dek_wrap_params,
    }))
}

#[put("/wrapped-dek", data = "<payload>")]
pub async fn update_wrapped_dek(pool: &State<PgPool>, user: CurrentUser, payload: Json<UpdateWrappedDekRequest>) -> Result<(), AppError> {
    let bytes = B64
        .decode(&payload.wrapped_dek)
        .map_err(|_| AppError::BadRequest("Invalid base64 in wrappedDek".to_string()))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.update_wrapped_dek(&user.id, &bytes, &payload.dek_wrap_params).await
}
