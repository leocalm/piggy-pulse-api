use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::database::user::UserRepository;
use crate::db::get_client;
use crate::error::app_error::AppError;
use crate::models::user::{LoginRequest, UserRequest, UserResponse};
use deadpool_postgres::Pool;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::serde::json::Json;
use rocket::{routes, State};
use uuid::Uuid;

#[rocket::post("/", data = "<payload>")]
pub async fn post_user(pool: &State<Pool>, payload: Json<UserRequest>) -> Result<(Status, Json<UserResponse>), AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let user = repo.get_user_by_email(&payload.email).await?;
    if user.is_some() {
        return Err(AppError::UserAlreadyExists(payload.email.clone()));
    }

    let user = repo.create_user(&payload.name, &payload.email, &payload.password).await?;
    Ok((Status::Created, Json(UserResponse::from(&user))))
}

#[rocket::put("/<id>", data = "<payload>")]
pub async fn put_user(pool: &State<Pool>, _current_user: CurrentUser, id: &str, payload: Json<UserRequest>) -> Result<Json<UserResponse>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    let user = repo.update_user(&uuid, &payload.name, &payload.email, &payload.password).await?;
    Ok(Json(UserResponse::from(&user)))
}

#[rocket::delete("/<id>")]
pub async fn delete_user_route(pool: &State<Pool>, _current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let uuid = Uuid::parse_str(id)?;
    repo.delete_user(&uuid).await?;
    Ok(Status::Ok)
}

#[rocket::post("/login", data = "<payload>")]
pub async fn post_user_login(pool: &State<Pool>, cookies: &CookieJar<'_>, payload: Json<LoginRequest>) -> Result<Status, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    if let Some(user) = repo.get_user_by_email(&payload.email).await? {
        repo.verify_password(&user, &payload.password).await?;
        let value = format!("{}:{}", user.id, user.email);
        cookies.add_private(Cookie::build(("user", value)).path("/").build());
    }

    Ok(Status::Ok)
}

#[rocket::post("/logout")]
pub fn post_user_logout(cookies: &CookieJar<'_>) -> Status {
    cookies.remove_private(Cookie::build("user").build());
    Status::Ok
}

pub fn routes() -> Vec<rocket::Route> {
    routes![post_user, post_user_login, post_user_logout, put_user, delete_user_route]
}
