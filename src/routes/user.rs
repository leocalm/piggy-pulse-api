use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::{AuthRateLimit, RateLimit};
use crate::models::user::{LoginRequest, UserRequest, UserResponse};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new user (sign up)
#[openapi(tag = "Users")]
#[post("/", data = "<payload>")]
pub async fn post_user(pool: &State<PgPool>, _rate_limit: AuthRateLimit, payload: Json<UserRequest>) -> Result<(Status, Json<UserResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let user = repo.get_user_by_email(&payload.email).await?;
    if user.is_some() {
        return Err(AppError::UserAlreadyExists(payload.email.clone()));
    }

    let user = repo.create_user(&payload.name, &payload.email, &payload.password).await?;
    Ok((Status::Created, Json(UserResponse::from(&user))))
}

/// Update a user by ID
#[openapi(tag = "Users")]
#[put("/<id>", data = "<payload>")]
pub async fn put_user(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    _current_user: CurrentUser,
    id: &str,
    payload: Json<UserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid user id", e))?;
    let user = repo.update_user(&uuid, &payload.name, &payload.email, &payload.password).await?;
    Ok(Json(UserResponse::from(&user)))
}

/// Delete a user by ID
#[openapi(tag = "Users")]
#[delete("/<id>")]
pub async fn delete_user_route(pool: &State<PgPool>, _rate_limit: RateLimit, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid user id", e))?;
    repo.delete_user(&uuid).await?;
    Ok(Status::Ok)
}

/// Log in a user and set authentication cookie
#[openapi(tag = "Users")]
#[post("/login", data = "<payload>")]
pub async fn post_user_login(
    pool: &State<PgPool>,
    _rate_limit: AuthRateLimit,
    cookies: &CookieJar<'_>,
    payload: Json<LoginRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    if let Some(user) = repo.get_user_by_email(&payload.email).await? {
        repo.verify_password(&user, &payload.password).await?;
        let value = format!("{}:{}", user.id, user.email);
        cookies.add_private(Cookie::build(("user", value)).path("/").build());
    }

    Ok(Status::Ok)
}

/// Log out the current user
#[openapi(tag = "Users")]
#[post("/logout")]
pub async fn post_user_logout(_rate_limit: RateLimit, cookies: &CookieJar<'_>) -> Status {
    cookies.remove_private(Cookie::build("user").build());
    Status::Ok
}

/// Get the currently authenticated user
#[openapi(tag = "Users")]
#[get("/me")]
pub async fn get_me(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    if let Some(user) = repo.get_user_by_id(&current_user.id).await? {
        Ok(Json(UserResponse::from(&user)))
    } else {
        Err(AppError::NotFound(current_user.id.to_string()))
    }
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![post_user, post_user_login, post_user_logout, put_user, delete_user_route, get_me]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_me_unauthorized_without_cookie() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/users/me").dispatch().await;

        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_me_returns_current_user() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let payload = serde_json::json!({
            "name": "Test User",
            "email": "test.user@example.com",
            "password": "password123"
        });

        let response = client.post("/api/users/").header(ContentType::JSON).body(payload.to_string()).dispatch().await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id");
        let user_email = user_json["email"].as_str().expect("user email");

        let cookie_value = format!("{}:{}", user_id, user_email);
        client.cookies().add_private(Cookie::build(("user", cookie_value)).path("/").build());

        let response = client.get("/api/users/me").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("me response body");
        let me_json: Value = serde_json::from_str(&body).expect("valid me json");

        assert_eq!(me_json["id"].as_str().unwrap(), user_id);
        assert_eq!(me_json["email"].as_str().unwrap(), user_email);
    }
}
