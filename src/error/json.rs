use rocket::data::{Data, FromData, Outcome};
use rocket::http::Status;
use rocket::request::Request;
use rocket::serde::json::serde_json;
use serde::de::DeserializeOwned;
use std::ops::Deref;
use tracing::warn;

/// A custom JSON wrapper that provides meaningful error logging when parsing fails.
///
/// Unlike Rocket's built-in `Json`, this wrapper logs structured information about
/// parse failures including the field name and expected type when possible.
#[derive(Debug)]
pub struct JsonBody<T>(pub T);

impl<T> Deref for JsonBody<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r, T: DeserializeOwned> FromData<'r> for JsonBody<T> {
    type Error = serde_json::Error;

    async fn from_data(req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r, Self> {
        let limit = req.limits().get("json").unwrap_or_else(|| 1.mebibytes());

        let bytes = match data.open(limit).into_bytes().await {
            Ok(bytes) if bytes.is_complete() => bytes.into_inner(),
            Ok(_) => {
                warn!(
                    method = %req.method(),
                    uri = %req.uri(),
                    "JSON payload exceeded size limit"
                );
                return Outcome::Error((
                    Status::PayloadTooLarge,
                    serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "payload too large")),
                ));
            }
            Err(e) => {
                warn!(
                    method = %req.method(),
                    uri = %req.uri(),
                    error = %e,
                    "Failed to read request body"
                );
                return Outcome::Error((Status::BadRequest, serde_json::Error::io(e)));
            }
        };

        match serde_json::from_slice::<T>(&bytes) {
            Ok(value) => Outcome::Success(JsonBody(value)),
            Err(e) => {
                let body_preview = String::from_utf8_lossy(&bytes);
                let body_preview = if body_preview.len() > 500 {
                    format!("{}...", &body_preview[..500])
                } else {
                    body_preview.to_string()
                };

                warn!(
                    method = %req.method(),
                    uri = %req.uri(),
                    error_message = %e,
                    error_line = e.line(),
                    error_column = e.column(),
                    error_category = ?e.classify(),
                    request_body = %body_preview,
                    "Failed to parse JSON request body"
                );

                Outcome::Error((Status::UnprocessableEntity, e))
            }
        }
    }
}

use rocket::data::ByteUnit;

trait ByteUnitExt {
    fn mebibytes(self) -> ByteUnit;
}

impl ByteUnitExt for u64 {
    fn mebibytes(self) -> ByteUnit {
        ByteUnit::Mebibyte(self)
    }
}
