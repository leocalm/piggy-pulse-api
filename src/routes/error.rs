use crate::middleware::rate_limit::RateLimitRetryAfter;
use rocket::http::{ContentType, Header, Status};
use rocket::response::{Responder, Result as ResponseResult};
use rocket::serde::Serialize;
use rocket::serde::json::Json;
use rocket::{Request, Response, catch};
use std::io::Cursor;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Error {
    pub message: String,
}

// no DB changes required in error routes

#[catch(404)]
pub fn not_found(_: &Request) -> Json<Error> {
    Json(Error {
        message: "Not found".to_string(),
    })
}

#[catch(409)]
pub fn conflict(_: &Request) -> Json<Error> {
    Json(Error {
        message: "Conflict".to_string(),
    })
}

pub struct TooManyRequests {
    pub retry_after: Option<u64>,
}

impl<'r> Responder<'r, 'static> for TooManyRequests {
    fn respond_to(self, _: &'r Request<'_>) -> ResponseResult<'static> {
        let body = serde_json::to_string(&Error {
            message: "Too Many Requests".to_string(),
        })
        .unwrap_or_else(|_| "{\"message\":\"Too Many Requests\"}".to_string());

        let mut response = Response::build();
        response.status(Status::TooManyRequests);
        response.header(ContentType::JSON);

        if let Some(retry_after) = self.retry_after {
            response.header(Header::new("Retry-After", retry_after.to_string()));
        }

        response.sized_body(body.len(), Cursor::new(body));
        response.ok()
    }
}

#[catch(429)]
pub fn too_many_requests(req: &Request) -> TooManyRequests {
    let retry_after = req.local_cache(|| None::<RateLimitRetryAfter>);
    TooManyRequests {
        retry_after: retry_after.as_ref().map(|value| value.0),
    }
}
