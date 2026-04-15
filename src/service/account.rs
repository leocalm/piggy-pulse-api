use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{
    AccountListResponse, AccountOptionListResponse, AccountOptionResponse, AccountStatus, AdjustBalanceRequest, CreateAccountRequest, EncryptedAccountResponse,
    UpdateAccountRequest, b64,
};
use crate::dto::common::PaginatedResponse;
use crate::error::app_error::AppError;
use crate::models::account::Account;
use uuid::Uuid;

pub struct AccountService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> AccountService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, user_id: &Uuid) -> Result<AccountListResponse, AppError> {
        let accounts = self.repository.list_accounts(user_id).await?;
        let total_count = accounts.len() as i64;
        let data: Vec<EncryptedAccountResponse> = accounts.iter().map(to_encrypted_response).collect();
        Ok(PaginatedResponse {
            data,
            total_count,
            has_more: false,
            next_cursor: None,
        })
    }

    pub async fn list_account_options(&self, user_id: &Uuid) -> Result<AccountOptionListResponse, AppError> {
        let accounts = self.repository.list_accounts(user_id).await?;
        Ok(accounts
            .iter()
            .filter(|a| !a.is_archived)
            .map(|a| AccountOptionResponse {
                id: a.id,
                account_type: a.account_type.into(),
                name_enc: b64(&a.name_enc),
                color_enc: b64(&a.color_enc),
            })
            .collect())
    }

    pub async fn get_account(&self, id: &Uuid, user_id: &Uuid) -> Result<EncryptedAccountResponse, AppError> {
        let account = self
            .repository
            .get_account_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;
        Ok(to_encrypted_response(&account))
    }

    pub async fn create_account(&self, request: &CreateAccountRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedAccountResponse, AppError> {
        let account = self.repository.create_account(request, user_id, dek).await?;
        Ok(to_encrypted_response(&account))
    }

    pub async fn update_account(&self, id: &Uuid, request: &UpdateAccountRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedAccountResponse, AppError> {
        let account = self.repository.update_account(id, request, user_id, dek).await?;
        Ok(to_encrypted_response(&account))
    }

    pub async fn delete_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.delete_account(id, user_id).await
    }

    pub async fn archive_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.archive_account(id, user_id).await
    }

    pub async fn unarchive_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.unarchive_account(id, user_id).await
    }

    pub async fn adjust_balance(&self, id: &Uuid, request: &AdjustBalanceRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedAccountResponse, AppError> {
        let account = self.repository.adjust_balance(id, request.new_balance, user_id, dek).await?;
        Ok(to_encrypted_response(&account))
    }
}

fn to_encrypted_response(account: &Account) -> EncryptedAccountResponse {
    EncryptedAccountResponse {
        id: account.id,
        account_type: account.account_type.into(),
        status: if account.is_archived {
            AccountStatus::Inactive
        } else {
            AccountStatus::Active
        },
        currency_id: account.currency_id,
        name_enc: b64(&account.name_enc),
        color_enc: b64(&account.color_enc),
        icon_enc: b64(&account.icon_enc),
        current_balance_enc: b64(&account.current_balance_enc),
        spend_limit_enc: account.spend_limit_enc.as_deref().map(b64),
        next_transfer_amount_enc: account.next_transfer_amount_enc.as_deref().map(b64),
        top_up_amount_enc: account.top_up_amount_enc.as_deref().map(b64),
        top_up_cycle: account.top_up_cycle.clone(),
        top_up_day: account.top_up_day,
        statement_close_day: account.statement_close_day,
        payment_due_day: account.payment_due_day,
    }
}
