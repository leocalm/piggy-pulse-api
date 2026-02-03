use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::vendor::{VendorOrderBy, VendorRepository};
use crate::error::app_error::AppError;
use crate::models::pagination::{PaginatedResponse, PaginationParams};
use crate::models::vendor::{VendorRequest, VendorResponse, VendorWithStatsResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, routes};
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_vendor(pool: &State<PgPool>, _current_user: CurrentUser, payload: Json<VendorRequest>) -> Result<(Status, Json<VendorResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let vendor = repo.create_vendor(&payload).await?;
    Ok((Status::Created, Json(VendorResponse::from(&vendor))))
}

#[rocket::get("/?<page>&<limit>")]
pub async fn list_all_vendors(
    pool: &State<PgPool>,
    _current_user: CurrentUser,
    page: Option<i64>,
    limit: Option<i64>,
) -> Result<Json<PaginatedResponse<VendorResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let pagination = if page.is_some() || limit.is_some() {
        Some(PaginationParams { page, limit })
    } else {
        None
    };

    let (vendors, total) = repo.list_vendors(pagination.as_ref()).await?;
    let responses: Vec<VendorResponse> = vendors.iter().map(VendorResponse::from).collect();

    let paginated = if let Some(params) = pagination {
        let effective_page = params.page.unwrap_or(1);
        let effective_limit = params.effective_limit().unwrap_or(PaginationParams::DEFAULT_LIMIT);
        PaginatedResponse::new(responses, effective_page, effective_limit, total)
    } else {
        PaginatedResponse::new(responses, 1, total, total)
    };

    Ok(Json(paginated))
}

#[rocket::get("/<id>")]
pub async fn get_vendor(pool: &State<PgPool>, _current_user: CurrentUser, id: &str) -> Result<Json<VendorResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    if let Some(vendor) = repo.get_vendor_by_id(&uuid).await? {
        Ok(Json(VendorResponse::from(&vendor)))
    } else {
        Err(AppError::NotFound("Vendor not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_vendor(pool: &State<PgPool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    repo.delete_vendor(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_vendor(pool: &State<PgPool>, _current_user: CurrentUser, id: &str, payload: Json<VendorRequest>) -> Result<Json<VendorResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let vendor = repo.update_vendor(&uuid, &payload).await?;
    Ok(Json(VendorResponse::from(&vendor)))
}

#[rocket::get("/with_status?<order_by>")]
pub async fn get_vendors_with_status(pool: &State<PgPool>, order_by: VendorOrderBy) -> Result<Json<Vec<VendorWithStatsResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(
        repo.list_vendors_with_status(order_by)
            .await?
            .iter()
            .map(VendorWithStatsResponse::from)
            .collect(),
    ))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![create_vendor, list_all_vendors, get_vendor, delete_vendor, put_vendor, get_vendors_with_status]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_create_vendor_validation_error() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let invalid_payload = serde_json::json!({
            "name": "AB",  // Too short
            "color": "#000"
        });

        let response = client
            .post("/api/vendors/")
            .header(ContentType::JSON)
            .body(invalid_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_vendor_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/vendors/not-valid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_vendor_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/vendors/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
