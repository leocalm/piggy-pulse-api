use rocket::State;
use rocket::get;
use rocket::http::Header;
use rocket::request::Request;
use rocket::response::{Responder, Response};
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::service::settings::SettingsService;

/// CSV response type for the export endpoint.
pub struct CsvResponse {
    body: String,
}

impl<'r> Responder<'r, 'static> for CsvResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let bytes = self.body.into_bytes();
        Response::build()
            .header(rocket::http::ContentType::new("text", "csv"))
            .header(Header::new("Content-Disposition", "attachment; filename=\"transactions.csv\""))
            .sized_body(bytes.len(), std::io::Cursor::new(bytes))
            .ok()
    }
}

#[get("/transactions")]
pub async fn export_transactions(pool: &State<PgPool>, user: CurrentUser) -> Result<CsvResponse, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = SettingsService::new(&repo);
    let csv = service.export_transactions_csv(&user.id).await?;
    Ok(CsvResponse { body: csv })
}
