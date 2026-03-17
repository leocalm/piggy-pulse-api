use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountResponse, CreateAccountRequest};
use crate::error::app_error::AppError;
use crate::models::account::AccountRequest;
use crate::service::account::AccountService;

#[post("/", data = "<payload>")]
pub async fn create_account(pool: &State<PgPool>, user: CurrentUser, payload: Json<CreateAccountRequest>) -> Result<(Status, Json<AccountResponse>), AppError> {
    let fields = payload.fields();
    fields.validate()?;

    // spend_limit is only valid for CreditCard and Allowance types
    if fields.spend_limit.is_some() && !matches!(&*payload, CreateAccountRequest::CreditCard(_) | CreateAccountRequest::Allowance(_)) {
        return Err(AppError::BadRequest(
            "spendLimit is only allowed for CreditCard and Allowance account types".to_string(),
        ));
    }

    let v1_request = AccountRequest {
        name: fields.name.clone(),
        color: fields.color.clone(),
        icon: "wallet".to_string(),
        account_type: payload.model_account_type(),
        balance: fields.initial_balance,
        spend_limit: fields.spend_limit.map(|s| s as i32),
        next_transfer_amount: None,
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    let response = service.create_account(&v1_request, &user.id).await?;
    Ok((Status::Created, Json(response)))
}
