use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::vendor::VendorOrderBy;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::pagination::{CursorPaginatedResponse, CursorParams};
use crate::models::vendor::{VendorRequest, VendorResponse, VendorWithPeriodStatsResponse, VendorWithStatsResponse};
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
pub async fn create_vendor(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<VendorRequest>,
) -> Result<(Status, Json<VendorResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let vendor = repo.create_vendor(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(VendorResponse::from(&vendor))))
}

/// List all vendors with cursor-based pagination and stats for a selected budget period.
/// Requires `period_id` query parameter.
#[openapi(tag = "Vendors")]
#[get("/?<period_id>&<cursor>&<limit>")]
pub async fn list_all_vendors(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    period_id: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
) -> Result<Json<CursorPaginatedResponse<VendorWithPeriodStatsResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let params = CursorParams::from_query(cursor, limit)?;
    let period_id = match period_id {
        Some(pid) => pid,
        None => return Err(AppError::BadRequest("Missing period_id".to_string())),
    };
    let period_uuid = Uuid::parse_str(&period_id).map_err(|e| AppError::uuid("Invalid period id", e))?;
    let period = repo.get_budget_period(&period_uuid, &current_user.id).await?;

    let vendors = repo.list_vendors(&params, &current_user.id, &period).await?;
    let responses: Vec<VendorWithPeriodStatsResponse> = vendors.iter().map(VendorWithPeriodStatsResponse::from).collect();
    Ok(Json(CursorPaginatedResponse::from_rows(responses, params.effective_limit(), |r| r.vendor.id)))
}

/// Get vendor options (list of all vendors for dropdowns/selection)
#[openapi(tag = "Vendors")]
#[get("/options")]
pub async fn get_vendor_options(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<VendorResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let vendors = repo.list_all_vendors(&current_user.id).await?;
    let responses: Vec<VendorResponse> = vendors.iter().map(VendorResponse::from).collect();
    Ok(Json(responses))
}

/// Get a vendor by ID
#[openapi(tag = "Vendors")]
#[get("/<id>")]
pub async fn get_vendor(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Json<VendorResponse>, AppError> {
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
pub async fn delete_vendor(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid vendor id", e))?;
    repo.delete_vendor(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a vendor by ID
#[openapi(tag = "Vendors")]
#[put("/<id>", data = "<payload>")]
pub async fn put_vendor(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<VendorRequest>,
) -> Result<Json<VendorResponse>, AppError> {
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
    _rate_limit: RateLimit,
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
    let (routes, spec) = rocket_okapi::openapi_get_routes_spec![
        create_vendor,
        list_all_vendors,
        get_vendor_options,
        get_vendor,
        delete_vendor,
        put_vendor,
        get_vendors_with_status
    ];
    (routes, spec)
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use chrono::{Duration, Utc};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

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
            .post("/api/v1/vendors/")
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

        let response = client.get("/api/v1/vendors/not-valid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_vendor_invalid_uuid() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.delete("/api/v1/vendors/invalid").dispatch().await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_vendors_includes_period_stats() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let user_payload = serde_json::json!({
            "name": "Test User",
            "email": "test.vendor@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(user_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_email = user_json["email"].as_str().expect("user email");

        let login_payload = serde_json::json!({
            "email": user_email,
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let login_response = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(login_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(login_response.status(), Status::Ok);

        let currency_payload = serde_json::json!({
            "name": "US Dollar",
            "symbol": "$",
            "currency": "USD",
            "decimal_places": 2
        });

        let response = client
            .post("/api/v1/currency/")
            .header(ContentType::JSON)
            .body(currency_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let account_payload = serde_json::json!({
            "name": "Checking",
            "color": "#000000",
            "icon": "bank",
            "account_type": "Checking",
            "currency": "USD",
            "balance": 1000,
            "spend_limit": null
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(account_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("account response body");
        let account_json: Value = serde_json::from_str(&body).expect("valid account json");
        let account_id = account_json["id"].as_str().expect("account id");

        let category_payload = serde_json::json!({
            "name": "Dining",
            "color": "#00FF00",
            "icon": "fork",
            "parent_id": null,
            "category_type": "Outgoing"
        });

        let response = client
            .post("/api/v1/categories/")
            .header(ContentType::JSON)
            .body(category_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("category response body");
        let category_json: Value = serde_json::from_str(&body).expect("valid category json");
        let category_id = category_json["id"].as_str().expect("category id");

        let vendor_payload = serde_json::json!({
            "name": "Vendor Co"
        });

        let response = client
            .post("/api/v1/vendors/")
            .header(ContentType::JSON)
            .body(vendor_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("vendor response body");
        let vendor_json: Value = serde_json::from_str(&body).expect("valid vendor json");
        let vendor_id = vendor_json["id"].as_str().expect("vendor id");

        let today = Utc::now().date_naive();
        let period_payload = serde_json::json!({
            "name": "Current Period",
            "start_date": (today - Duration::days(1)).to_string(),
            "end_date": (today + Duration::days(1)).to_string()
        });

        let response = client
            .post("/api/v1/budget_period/")
            .header(ContentType::JSON)
            .body(period_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let period_id = response.into_string().await.expect("period id");

        let tx_payload = serde_json::json!({
            "amount": 250,
            "description": "Lunch",
            "occurred_at": today.to_string(),
            "category_id": category_id,
            "from_account_id": account_id,
            "to_account_id": null,
            "vendor_id": vendor_id
        });

        let response = client
            .post("/api/v1/transactions/")
            .header(ContentType::JSON)
            .body(tx_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let response = client.get(format!("/api/v1/vendors/?period_id={}&limit=50", period_id)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("vendors response body");
        let list_json: Value = serde_json::from_str(&body).expect("valid vendors json");
        let data = list_json["data"].as_array().expect("data array");
        let vendor = data.iter().find(|item| item["id"].as_str() == Some(vendor_id)).expect("vendor in list");

        let expected_last_used = today.to_string();
        assert_eq!(vendor["transaction_count"].as_i64(), Some(1));
        assert_eq!(vendor["last_used_at"].as_str(), Some(expected_last_used.as_str()));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_list_vendors_missing_period_id() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let user_payload = serde_json::json!({
            "name": "Test User",
            "email": "test.vendor.missing@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(user_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_email = user_json["email"].as_str().expect("user email");

        let login_payload = serde_json::json!({
            "email": user_email,
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let login_response = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(login_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(login_response.status(), Status::Ok);

        let response = client.get("/api/v1/vendors/?limit=50").dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_vendor_options() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/vendors/options").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
    }
}
