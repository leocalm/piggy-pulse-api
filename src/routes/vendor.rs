use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::vendor::VendorOrderBy;
use crate::error::app_error::AppError;
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::models::vendor::{VendorRequest, VendorResponse, VendorWithStatsResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new vendor
#[openapi(tag = "Vendors")]
#[post("/", data = "<payload>")]
pub async fn create_vendor(pool: &State<PgPool>, current_user: CurrentUser, payload: Json<VendorRequest>) -> Result<(Status, Json<VendorResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let vendor = repo.create_vendor(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(VendorResponse::from(&vendor))))
}

/// List all vendors with cursor-based pagination
#[openapi(tag = "Vendors")]
#[get("/?<cursor>&<limit>")]
pub async fn list_all_vendors(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<VendorResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;

    let vendors = repo.list_vendors(&params, &current_user.id).await?;
    let responses: Vec<VendorResponse> = vendors.iter().map(VendorResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.id)))
}

/// Get a vendor by ID
#[openapi(tag = "Vendors")]
#[get("/<id>")]
pub async fn get_vendor(pool: &State<PgPool>, current_user: CurrentUser, id: &str) -> Result<Json<VendorResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    if let Some(vendor) = repo.get_vendor_by_id(&uuid, &current_user.id).await? {
        Ok(Json(VendorResponse::from(&vendor)))
    } else {
        Err(AppError::NotFound("Vendor not found".to_string()))
    }
}

/// Delete a vendor by ID
#[openapi(tag = "Vendors")]
#[delete("/<id>")]
pub async fn delete_vendor(pool: &State<PgPool>, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    repo.delete_vendor(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a vendor by ID
#[openapi(tag = "Vendors")]
#[put("/<id>", data = "<payload>")]
pub async fn put_vendor(pool: &State<PgPool>, current_user: CurrentUser, id: &str, payload: Json<VendorRequest>) -> Result<Json<VendorResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    let vendor = repo.update_vendor(&uuid, &payload, &current_user.id).await?;
    Ok(Json(VendorResponse::from(&vendor)))
}

/// Get vendors with transaction statistics, ordered by specified field
#[openapi(tag = "Vendors")]
#[get("/with_status?<order_by>")]
pub async fn get_vendors_with_status(
    pool: &State<PgPool>,
    current_user: CurrentUser,
    order_by: VendorOrderBy,
) -> Result<Json<Vec<VendorWithStatsResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    Ok(Json(
        repo.list_vendors_with_status(order_by, &current_user.id)
            .await?
            .iter()
            .map(VendorWithStatsResponse::from)
            .collect(),
    ))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![create_vendor, list_all_vendors, get_vendor, delete_vendor, put_vendor, get_vendors_with_status]
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
