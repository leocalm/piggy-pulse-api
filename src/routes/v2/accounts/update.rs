use rocket::State;
use rocket::put;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountResponse, UpdateAccountRequest};
use crate::error::app_error::AppError;
use crate::models::account::AccountUpdateRequest;

#[put("/<id>", data = "<payload>")]
pub async fn update_account(pool: &State<PgPool>, user: CurrentUser, id: &str, payload: Json<UpdateAccountRequest>) -> Result<Json<AccountResponse>, AppError> {
    let fields = payload.fields();
    fields.validate()?;

    // spend_limit is only valid for CreditCard and Allowance types
    if fields.spend_limit.is_some() && !matches!(&*payload, UpdateAccountRequest::CreditCard(_) | UpdateAccountRequest::Allowance(_)) {
        return Err(AppError::BadRequest(
            "spendLimit is only allowed for CreditCard and Allowance account types".to_string(),
        ));
    }

    // top_up fields are only valid for Allowance accounts
    let is_allowance = matches!(&*payload, UpdateAccountRequest::Allowance(_));
    if !is_allowance && (fields.top_up_amount.is_some() || fields.top_up_cycle.is_some() || fields.top_up_day.is_some()) {
        return Err(AppError::BadRequest(
            "topUpAmount, topUpCycle and topUpDay are only allowed for Allowance account type".to_string(),
        ));
    }

    // statement/payment fields are only valid for CreditCard accounts
    let is_credit_card = matches!(&*payload, UpdateAccountRequest::CreditCard(_));
    if !is_credit_card && (fields.statement_close_day.is_some() || fields.payment_due_day.is_some()) {
        return Err(AppError::BadRequest(
            "statementCloseDay and paymentDueDay are only allowed for CreditCard account type".to_string(),
        ));
    }

    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid account id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let v1_request = AccountUpdateRequest {
        name: fields.name.clone(),
        color: fields.color.clone(),
        icon: "wallet".to_string(),
        account_type: payload.model_account_type(),
        balance: Option::from(fields.initial_balance),
        spend_limit: fields.spend_limit.map(|s| s as i32),
        next_transfer_amount: None,
        top_up_amount: if is_allowance { fields.top_up_amount } else { None },
        top_up_cycle: if is_allowance { fields.top_up_cycle.clone() } else { None },
        top_up_day: if is_allowance { fields.top_up_day } else { None },
        statement_close_day: if is_credit_card { fields.statement_close_day } else { None },
        payment_due_day: if is_credit_card { fields.payment_due_day } else { None },
    };

    let account = repo.update_account(&uuid, &v1_request, &user.id).await?;
    Ok(Json(AccountResponse::from(&account)))
}
