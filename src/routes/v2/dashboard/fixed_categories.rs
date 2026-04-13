use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::dashboard::FixedCategoriesResponseEither;
use crate::error::app_error::AppError;
use crate::service::dashboard::DashboardService;

#[get("/fixed-categories?<periodId>&<responseFormat>")]
pub async fn get_fixed_categories(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
    #[allow(non_snake_case)] responseFormat: Option<String>,
) -> Result<Json<FixedCategoriesResponseEither>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?,
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = DashboardService::new(&repo);

    if responseFormat.as_deref() == Some("wrapped") {
        let wrapped = service.get_fixed_categories_wrapped(&period_uuid, &user.id).await?;
        Ok(Json(FixedCategoriesResponseEither::Wrapped(wrapped)))
    } else {
        let legacy = service.get_fixed_categories(&period_uuid, &user.id).await?;
        Ok(Json(FixedCategoriesResponseEither::Legacy(legacy)))
    }
}
