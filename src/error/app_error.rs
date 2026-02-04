use rocket::http::Status;
use rocket::response::Responder;
use rocket::{Request, Response};
use rocket_okapi::OpenApiError;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use std::io::Cursor;
use thiserror::Error;
use tracing::error;
use validator::ValidationErrors;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Internal server error")]
    Db {
        message: String,
        #[source]
        source: sqlx::error::Error,
    },
    #[error("User not found")]
    UserNotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Internal server error")]
    PasswordHash { message: String },
    #[error("User {0} already exists")]
    UserAlreadyExists(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Currency not found: {0}")]
    CurrencyDoesNotExist(String),
    #[error("Internal server error")]
    UuidError {
        message: String,
        #[source]
        source: uuid::Error,
    },
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Internal server error")]
    ConfigurationError {
        message: String,
        #[source]
        source: figment::Error,
    },
}

impl AppError {
    pub fn db(message: impl Into<String>, source: sqlx::error::Error) -> Self {
        Self::Db {
            message: message.into(),
            source,
        }
    }

    pub fn uuid(message: impl Into<String>, source: uuid::Error) -> Self {
        Self::UuidError {
            message: message.into(),
            source,
        }
    }

    pub fn password_hash(message: impl Into<String>, source: password_hash::Error) -> Self {
        Self::PasswordHash {
            message: format!("{}: {}", message.into(), source),
        }
    }
}

impl From<password_hash::Error> for AppError {
    fn from(e: password_hash::Error) -> Self {
        AppError::password_hash("Password hashing failed", e)
    }
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        AppError::uuid("Invalid UUID", e)
    }
}

impl From<&AppError> for Status {
    fn from(e: &AppError) -> Self {
        match e {
            AppError::UserNotFound => Status::NotFound,
            AppError::InvalidCredentials => Status::Forbidden,
            AppError::PasswordHash { .. } => Status::InternalServerError,
            AppError::Db { .. } => Status::InternalServerError,
            AppError::Unauthorized => Status::Unauthorized,
            AppError::UserAlreadyExists(_) => Status::Conflict,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::NotFound(_) => Status::NotFound,
            AppError::CurrencyDoesNotExist(_) => Status::BadRequest,
            AppError::UuidError { .. } => Status::BadRequest,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::ConfigurationError { .. } => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &Request<'_>) -> rocket::response::Result<'static> {
        // Extract request context for better error logging
        let method = req.method();
        let uri = req.uri();

        // Try to get request_id from local_cache
        let request_id = req
            .local_cache(|| None::<crate::middleware::RequestId>)
            .as_ref()
            .map(|r| r.0.as_str())
            .unwrap_or("unknown");

        // Try to get user from auth
        let user_id = req
            .local_cache(|| None::<crate::auth::CurrentUser>)
            .as_ref()
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        error!(
            error = ?self,
            request_id = %request_id,
            user_id = %user_id,
            method = %method,
            uri = %uri,
            "request failed"
        );

        let status = Status::from(&self);
        let body = self.to_string();

        Response::build().status(status).sized_body(body.len(), Cursor::new(body)).ok()
    }
}

impl OpenApiResponderInner for AppError {
    fn responses(_gen: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiResponse};
        let mut responses = Responses::default();
        responses.responses.insert(
            "400".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Bad Request".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "401".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Unauthorized".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "404".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Not Found".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "500".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Internal Server Error".to_string(),
                ..Default::default()
            }),
        );
        Ok(responses)
    }
}

impl From<figment::Error> for AppError {
    fn from(e: figment::Error) -> Self {
        AppError::ConfigurationError {
            message: "Failed to read configuration".to_string(),
            source: e,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            _ => AppError::db("Database error", e),
        }
    }
}
