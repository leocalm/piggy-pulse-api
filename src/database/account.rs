use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{CreateAccountRequest, UpdateAccountRequest};
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountType};
use uuid::Uuid;

impl PostgresRepository {
    /// Encrypt every plaintext field on the request and insert a new
    /// `account` row. Name uniqueness is enforced in Rust: the server
    /// fetches every existing account's `name_enc`, decrypts with the
    /// caller's DEK, and checks for a case-insensitive match before
    /// the INSERT. Runs inside a tx that locks the `users` row so two
    /// concurrent creates from the same session serialize.
    pub async fn create_account(&self, request: &CreateAccountRequest, user_id: &Uuid, dek: &Dek) -> Result<Account, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_account_name_unique(&mut tx, dek, user_id, &request.name, None).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let color_enc = dek.encrypt_string(&request.color)?;
        let current_balance_enc = dek.encrypt_i64(request.initial_balance)?;
        let spend_limit_enc = request.spend_limit.map(|v| dek.encrypt_i64(v)).transpose()?;
        let next_transfer_amount_enc = request.next_transfer_amount.map(|v| dek.encrypt_i64(v)).transpose()?;
        let top_up_amount_enc = request.top_up_amount.map(|v| dek.encrypt_i64(v)).transpose()?;

        let account: Account = sqlx::query_as(
            r#"
INSERT INTO account (
    id, user_id, account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
) VALUES (
    gen_random_uuid(), $1, $2::text::account_type, $3, false,
    $4, $5, $6,
    $7, $8, $9,
    $10, $11, $12, $13
)
RETURNING
    id, account_type::text AS account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
"#,
        )
        .bind(user_id)
        .bind(account_type_to_db(request.account_type.into()))
        .bind(request.currency_id)
        .bind(&name_enc)
        .bind(&color_enc)
        .bind(&current_balance_enc)
        .bind(spend_limit_enc.as_deref())
        .bind(next_transfer_amount_enc.as_deref())
        .bind(top_up_amount_enc.as_deref())
        .bind(request.top_up_cycle.as_deref())
        .bind(request.top_up_day)
        .bind(request.statement_close_day)
        .bind(request.payment_due_day)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(account)
    }

    pub async fn get_account_by_id(&self, id: &Uuid, user_id: &Uuid) -> Result<Option<Account>, AppError> {
        let account = sqlx::query_as::<_, Account>(
            r#"
SELECT id, account_type::text AS account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
FROM account
WHERE id = $1 AND user_id = $2
"#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(account)
    }

    /// List all accounts for a user. Pagination is applied client-side
    /// because the server can't meaningfully sort or filter by any
    /// encrypted field; the whole list comes back on every call.
    pub async fn list_accounts(&self, user_id: &Uuid) -> Result<Vec<Account>, AppError> {
        let accounts = sqlx::query_as::<_, Account>(
            r#"
SELECT id, account_type::text AS account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
FROM account
WHERE user_id = $1
ORDER BY id
"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(accounts)
    }

    pub async fn update_account(&self, id: &Uuid, request: &UpdateAccountRequest, user_id: &Uuid, dek: &Dek) -> Result<Account, AppError> {
        let mut tx = self.pool.begin().await?;

        lock_user_row(&mut tx, user_id).await?;
        check_account_name_unique(&mut tx, dek, user_id, &request.name, Some(id)).await?;

        let name_enc = dek.encrypt_string(&request.name)?;
        let color_enc = dek.encrypt_string(&request.color)?;
        let spend_limit_enc = request.spend_limit.map(|v| dek.encrypt_i64(v)).transpose()?;
        let next_transfer_amount_enc = request.next_transfer_amount.map(|v| dek.encrypt_i64(v)).transpose()?;
        let top_up_amount_enc = request.top_up_amount.map(|v| dek.encrypt_i64(v)).transpose()?;

        let account: Account = sqlx::query_as(
            r#"
UPDATE account
SET account_type = $1::text::account_type,
    currency_id = $2,
    name_enc = $3,
    color_enc = $4,
    spend_limit_enc = $5,
    next_transfer_amount_enc = $6,
    top_up_amount_enc = $7,
    top_up_cycle = $8,
    top_up_day = $9,
    statement_close_day = $10,
    payment_due_day = $11
WHERE id = $12 AND user_id = $13
RETURNING
    id, account_type::text AS account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
"#,
        )
        .bind(account_type_to_db(request.account_type.into()))
        .bind(request.currency_id)
        .bind(&name_enc)
        .bind(&color_enc)
        .bind(spend_limit_enc.as_deref())
        .bind(next_transfer_amount_enc.as_deref())
        .bind(top_up_amount_enc.as_deref())
        .bind(request.top_up_cycle.as_deref())
        .bind(request.top_up_day)
        .bind(request.statement_close_day)
        .bind(request.payment_due_day)
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        tx.commit().await?;
        Ok(account)
    }

    pub async fn delete_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM account WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Account not found".to_string()));
        }
        Ok(())
    }

    pub async fn archive_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE account SET is_archived = true WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Account not found".to_string()));
        }
        Ok(())
    }

    pub async fn unarchive_account(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let result = sqlx::query("UPDATE account SET is_archived = false WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Account not found".to_string()));
        }
        Ok(())
    }

    /// Set `current_balance_enc` to an absolute value. Uses SELECT FOR
    /// UPDATE on the row so concurrent transaction inserts that also
    /// maintain `current_balance_enc` serialize behind us.
    pub async fn adjust_balance(&self, id: &Uuid, new_balance: i64, user_id: &Uuid, dek: &Dek) -> Result<Account, AppError> {
        let mut tx = self.pool.begin().await?;

        let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM account WHERE id = $1 AND user_id = $2 FOR UPDATE")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound("Account not found".to_string()));
        }

        let new_enc = dek.encrypt_i64(new_balance)?;

        let account: Account = sqlx::query_as(
            r#"
UPDATE account
SET current_balance_enc = $1
WHERE id = $2 AND user_id = $3
RETURNING
    id, account_type::text AS account_type, currency_id, is_archived,
    name_enc, color_enc, current_balance_enc,
    spend_limit_enc, next_transfer_amount_enc, top_up_amount_enc,
    top_up_cycle, top_up_day, statement_close_day, payment_due_day
"#,
        )
        .bind(&new_enc)
        .bind(id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(account)
    }
}

async fn lock_user_row(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, user_id: &Uuid) -> Result<(), AppError> {
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(())
}

/// Fetch every existing account's `name_enc` for this user, decrypt,
/// and reject if the new name matches an existing one (optionally
/// excluding the account being updated). Case-insensitive.
async fn check_account_name_unique(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    dek: &Dek,
    user_id: &Uuid,
    candidate: &str,
    exclude_id: Option<&Uuid>,
) -> Result<(), AppError> {
    let rows: Vec<(Uuid, Vec<u8>)> = sqlx::query_as("SELECT id, name_enc FROM account WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(&mut **tx)
        .await?;

    let candidate_lower = candidate.to_lowercase();
    for (row_id, name_enc) in rows {
        if exclude_id.is_some_and(|id| *id == row_id) {
            continue;
        }
        let existing = dek.decrypt_string(&name_enc)?;
        if existing.to_lowercase() == candidate_lower {
            return Err(AppError::Conflict(format!("An account named '{}' already exists", candidate)));
        }
    }
    Ok(())
}

pub fn account_type_to_db(account_type: AccountType) -> String {
    match account_type {
        AccountType::Checking => "Checking".to_string(),
        AccountType::Savings => "Savings".to_string(),
        AccountType::CreditCard => "CreditCard".to_string(),
        AccountType::Wallet => "Wallet".to_string(),
        AccountType::Allowance => "Allowance".to_string(),
    }
}

impl From<crate::dto::accounts::AccountType> for AccountType {
    fn from(t: crate::dto::accounts::AccountType) -> Self {
        match t {
            crate::dto::accounts::AccountType::Checking => AccountType::Checking,
            crate::dto::accounts::AccountType::Savings => AccountType::Savings,
            crate::dto::accounts::AccountType::CreditCard => AccountType::CreditCard,
            crate::dto::accounts::AccountType::Wallet => AccountType::Wallet,
            crate::dto::accounts::AccountType::Allowance => AccountType::Allowance,
        }
    }
}

impl From<AccountType> for crate::dto::accounts::AccountType {
    fn from(t: AccountType) -> Self {
        match t {
            AccountType::Checking => crate::dto::accounts::AccountType::Checking,
            AccountType::Savings => crate::dto::accounts::AccountType::Savings,
            AccountType::CreditCard => crate::dto::accounts::AccountType::CreditCard,
            AccountType::Wallet => crate::dto::accounts::AccountType::Wallet,
            AccountType::Allowance => crate::dto::accounts::AccountType::Allowance,
        }
    }
}
