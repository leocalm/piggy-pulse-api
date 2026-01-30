use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::vendor::{VendorOrderBy, VendorRepository};
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::vendor::{VendorRequest, VendorResponse, VendorWithStatsResponse};
use deadpool_postgres::Pool;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;
use validator::Validate;

#[rocket::post("/", data = "<payload>")]
pub async fn create_vendor(pool: &State<Pool>, _current_user: CurrentUser, payload: Json<VendorRequest>) -> Result<(Status, Json<VendorResponse>), AppError> {
    payload.validate()?;

    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let vendor = repo.create_vendor(&payload).await?;
    Ok((Status::Created, Json(VendorResponse::from(&vendor))))
}

#[rocket::get("/")]
pub async fn list_all_vendors(pool: &State<Pool>, _current_user: CurrentUser) -> Result<Json<Vec<VendorResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let vendors = repo.list_vendors().await?;
    Ok(Json(vendors.iter().map(VendorResponse::from).collect()))
}

#[rocket::get("/<id>")]
pub async fn get_vendor(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Json<VendorResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    if let Some(vendor) = repo.get_vendor_by_id(&uuid).await? {
        Ok(Json(VendorResponse::from(&vendor)))
    } else {
        Err(AppError::NotFound("Vendor not found".to_string()))
    }
}

#[rocket::delete("/<id>")]
pub async fn delete_vendor(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_vendor(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_vendor(pool: &State<Pool>, _current_user: CurrentUser, id: &str, payload: Json<VendorRequest>) -> Result<Json<VendorResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let vendor = repo.update_vendor(&uuid, &payload).await?;
    Ok(Json(VendorResponse::from(&vendor)))
}

#[rocket::get("/with_status?<order_by>")]
pub async fn get_vendors_with_status(pool: &State<Pool>, order_by: VendorOrderBy) -> Result<Json<Vec<VendorWithStatsResponse>>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
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
