use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::period::PeriodGapsResponse;
use crate::error::app_error::AppError;
use crate::service::period::PeriodService;

#[get("/gaps")]
pub async fn get_period_gaps(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<PeriodGapsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = PeriodService::new(&repo);
    let gaps = service.get_gaps(&user.id).await?;
    Ok(Json(gaps))
}
