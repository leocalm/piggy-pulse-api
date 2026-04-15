//! Transaction write path — encryption-at-rest edition.
//!
//! Phase 2b of the encryption refactor. See
//! `.kiro/specs/encryption-at-rest/design.md` for the design.
//!
//! This module owns:
//!   * validation of foreign-key ownership on the write path
//!   * encryption of `amount` and `description` under the session DEK
//!   * insertion of ledger rows with `amount_enc` + `description_enc`
//!   * maintenance of `logical_transaction_state.current_sum_enc` +
//!     Rust-side `is_effective` on every insert
//!   * read-modify-write of `account.current_balance_enc` under
//!     `SELECT ... FOR UPDATE` so concurrent writers serialize cleanly
//!
//! Read paths are intentionally NOT in this module anymore. Phase 3 will
//! introduce a narrow `GET /v2/transactions` endpoint that returns the
//! full period as ciphertext and let the client derive everything else.
//! Until Phase 3 lands, read-side queries return `unimplemented!()` or
//! are deleted outright at the call sites.

use crate::crypto::Dek;
use crate::database::category::category_type_from_db;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::category::CategoryType;
use crate::models::transaction::TransactionRequest;
use chrono::NaiveDate;
use uuid::Uuid;

/// Per-logical-transaction running state. The `current_sum` is stored
/// encrypted on disk as `current_sum_enc BYTEA`; reading it into Rust as a
/// plaintext i64 requires decryption with the user's DEK.
///
/// `is_effective` is a plain Boolean written by the Rust service layer on
/// every insert (no longer a GENERATED column, because generating it would
/// require the database to read plaintext `current_sum`). Every insert
/// computes the new effective flag in Rust and writes it alongside the
/// encrypted sum.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct LogicalTransactionState {
    pub id: Uuid,
    pub user_id: Uuid,
    pub current_sum_enc: Vec<u8>,
    pub is_effective: bool,
    pub latest_seq: i64,
    pub first_created_at: chrono::DateTime<chrono::Utc>,
}

/// Snapshot of the Latest_Row's metadata columns. `description_enc` and
/// `amount_enc` are carried as raw ciphertext so that void/correct paths
/// can copy them verbatim into the compensating row without decrypting —
/// the trigger sees the same bytes, the server never sees the plaintext.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct LatestRowSnapshot {
    pub amount_enc: Vec<u8>,
    pub description_enc: Vec<u8>,
    pub occurred_at: NaiveDate,
    pub category_id: Option<Uuid>,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
}

/// Minimal return type for a write operation. The client already has all
/// joined metadata cached locally, so the server only returns the
/// structural ids + timestamps + ciphertext that might be new.
#[derive(Debug, Clone)]
pub struct LedgerInsertResult {
    pub id: Uuid,
    pub seq: i64,
    pub first_created_at: chrono::DateTime<chrono::Utc>,
    pub occurred_at: NaiveDate,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub amount_enc: Vec<u8>,
    pub description_enc: Vec<u8>,
}

impl PostgresRepository {
    // ─────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────

    async fn validate_transaction_ownership(&self, transaction: &TransactionRequest, user_id: &Uuid) -> Result<(), AppError> {
        let category_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM category WHERE id = $1 AND user_id = $2)")
            .bind(transaction.category_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        if !category_exists {
            return Err(AppError::BadRequest("Invalid category_id for current user".to_string()));
        }

        let from_account_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND user_id = $2)")
            .bind(transaction.from_account_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        if !from_account_exists {
            return Err(AppError::BadRequest("Invalid from_account_id for current user".to_string()));
        }

        if let Some(to_account_id) = transaction.to_account_id {
            let to_account_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account WHERE id = $1 AND user_id = $2)")
                .bind(to_account_id)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
            if !to_account_exists {
                return Err(AppError::BadRequest("Invalid to_account_id for current user".to_string()));
            }
        }

        if let Some(vendor_id) = transaction.vendor_id {
            let vendor_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM vendor WHERE id = $1 AND user_id = $2)")
                .bind(vendor_id)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
            if !vendor_exists {
                return Err(AppError::BadRequest("Invalid vendor_id for current user".to_string()));
            }
        }

        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────

    /// Lock the `logical_transaction_state` row for a given logical
    /// transaction. Returns `None` if the id does not exist for this
    /// user — caller maps this to 404. Must be called inside an open
    /// database transaction.
    pub(super) async fn lock_logical_transaction_state(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<LogicalTransactionState>, AppError> {
        let row = sqlx::query_as::<_, LogicalTransactionState>(
            r#"
            SELECT id, user_id, current_sum_enc, is_effective, latest_seq, first_created_at
              FROM logical_transaction_state
             WHERE id = $1 AND user_id = $2
             FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        Ok(row)
    }

    /// Fetch the Latest_Row's metadata + ciphertext bytes. Used by
    /// void/correct to copy fields into compensating rows without
    /// decrypting the description.
    pub(super) async fn fetch_latest_row(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: &Uuid,
        latest_seq: i64,
    ) -> Result<LatestRowSnapshot, AppError> {
        let row = sqlx::query_as::<_, LatestRowSnapshot>(
            r#"
            SELECT amount_enc, description_enc, occurred_at, category_id,
                   from_account_id, to_account_id, vendor_id
              FROM transaction
             WHERE id = $1 AND seq = $2
            "#,
        )
        .bind(id)
        .bind(latest_seq)
        .fetch_one(&mut **tx)
        .await?;
        Ok(row)
    }

    /// Insert a ledger row with pre-encrypted `amount_enc` and
    /// `description_enc`. Returns the inserted row's `seq` so callers can
    /// update `latest_seq` in `logical_transaction_state`.
    ///
    /// `id = None` generates a fresh UUID (for brand-new logical
    /// transactions); `id = Some(_)` reuses an existing logical id (for
    /// void and correct compensating rows).
    #[allow(clippy::too_many_arguments)]
    async fn insert_ledger_row_enc_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Option<&Uuid>,
        user_id: &Uuid,
        amount_enc: &[u8],
        description_enc: &[u8],
        occurred_at: NaiveDate,
        category_id: Option<&Uuid>,
        from_account_id: &Uuid,
        to_account_id: Option<&Uuid>,
        vendor_id: Option<&Uuid>,
    ) -> Result<(Uuid, i64, chrono::DateTime<chrono::Utc>), AppError> {
        #[derive(sqlx::FromRow)]
        struct InsertedRow {
            id: Uuid,
            seq: i64,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let row: InsertedRow = sqlx::query_as(
            r#"
            INSERT INTO transaction (
                id, user_id, amount_enc, description_enc, occurred_at,
                category_id, from_account_id, to_account_id, vendor_id
            )
            VALUES (COALESCE($1, gen_random_uuid()), $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, seq, created_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(amount_enc)
        .bind(description_enc)
        .bind(occurred_at)
        .bind(category_id)
        .bind(from_account_id)
        .bind(to_account_id)
        .bind(vendor_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok((row.id, row.seq, row.created_at))
    }

    /// Upsert `logical_transaction_state` for a given logical id. On first
    /// insert, seeds the row with `current_sum = amount`, `is_effective =
    /// (amount != 0)`, `latest_seq = seq`, `first_created_at = created_at`.
    /// On subsequent inserts (compensating / correction rows), reads the
    /// existing encrypted sum, adds the delta, and re-writes.
    ///
    /// Returns the new `current_sum` after the delta is applied, for
    /// callers that need it (e.g. void needs to write encrypt(0)).
    #[allow(clippy::too_many_arguments)]
    async fn upsert_lts_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        dek: &Dek,
        id: &Uuid,
        user_id: &Uuid,
        delta: i64,
        seq: i64,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64, AppError> {
        #[derive(sqlx::FromRow)]
        struct ExistingLts {
            current_sum_enc: Vec<u8>,
        }

        let existing: Option<ExistingLts> = sqlx::query_as("SELECT current_sum_enc FROM logical_transaction_state WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;

        let new_sum: i64 = match existing {
            Some(row) => {
                let prev = dek.decrypt_i64(&row.current_sum_enc)?;
                let new = prev
                    .checked_add(delta)
                    .ok_or_else(|| AppError::BadRequest("current_sum overflow".to_string()))?;
                let new_enc = dek.encrypt_i64(new)?;
                sqlx::query(
                    "UPDATE logical_transaction_state
                        SET current_sum_enc = $1,
                            is_effective = $2,
                            latest_seq = $3
                      WHERE id = $4",
                )
                .bind(&new_enc)
                .bind(new != 0)
                .bind(seq)
                .bind(id)
                .execute(&mut **tx)
                .await?;
                new
            }
            None => {
                let new_enc = dek.encrypt_i64(delta)?;
                sqlx::query(
                    "INSERT INTO logical_transaction_state
                        (id, user_id, current_sum_enc, is_effective, latest_seq, first_created_at)
                      VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(id)
                .bind(user_id)
                .bind(&new_enc)
                .bind(delta != 0)
                .bind(seq)
                .bind(created_at)
                .execute(&mut **tx)
                .await?;
                delta
            }
        };

        Ok(new_sum)
    }

    /// Apply a signed delta to `account.current_balance_enc` under a
    /// row-level lock. Decrypts the existing value (lazily initializing
    /// from 0 if the account has no encrypted balance yet), adds the
    /// delta, re-encrypts, writes back.
    async fn apply_account_balance_delta(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account_id: &Uuid,
        delta: i64,
        dek: &Dek,
    ) -> Result<(), AppError> {
        if delta == 0 {
            return Ok(());
        }

        let row: (Vec<u8>,) = sqlx::query_as("SELECT current_balance_enc FROM account WHERE id = $1 FOR UPDATE")
            .bind(account_id)
            .fetch_one(&mut **tx)
            .await?;

        let prev: i64 = dek.decrypt_i64(&row.0)?;
        let new = prev
            .checked_add(delta)
            .ok_or_else(|| AppError::BadRequest("account balance overflow".to_string()))?;
        let new_enc = dek.encrypt_i64(new)?;

        sqlx::query("UPDATE account SET current_balance_enc = $1 WHERE id = $2")
            .bind(&new_enc)
            .bind(account_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Apply the balance-side effect of a ledger insertion. The delta
    /// depends on the category type:
    ///
    ///   * Incoming: credit `from_account` with `+amount`
    ///   * Outgoing: debit `from_account` with `-amount`
    ///   * Transfer: debit `from_account` + credit `to_account`
    ///   * Uncategorized (NULL category_id): no balance effect
    ///
    /// Matches the semantics the ledger refactor's Phase 1 trigger used,
    /// now in Rust because the trigger can't read plaintext amounts.
    async fn apply_category_balance_effect(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        cat_type: Option<CategoryType>,
        amount: i64,
        from_account_id: &Uuid,
        to_account_id: Option<&Uuid>,
        dek: &Dek,
    ) -> Result<(), AppError> {
        match cat_type {
            Some(CategoryType::Incoming) => {
                self.apply_account_balance_delta(tx, from_account_id, amount, dek).await?;
            }
            Some(CategoryType::Outgoing) => {
                self.apply_account_balance_delta(tx, from_account_id, -amount, dek).await?;
            }
            Some(CategoryType::Transfer) => {
                self.apply_account_balance_delta(tx, from_account_id, -amount, dek).await?;
                if let Some(to) = to_account_id {
                    self.apply_account_balance_delta(tx, to, amount, dek).await?;
                }
            }
            None => {
                // Uncategorized — no balance effect, matching legacy behavior.
            }
        }
        Ok(())
    }

    /// Resolve a category's type so we know which balance delta to apply.
    /// `None` is returned for uncategorized transactions.
    async fn resolve_category_type(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        category_id: Option<&Uuid>,
    ) -> Result<Option<CategoryType>, AppError> {
        let Some(cat_id) = category_id else { return Ok(None) };
        let row: Option<(String,)> = sqlx::query_as("SELECT category_type::text FROM category WHERE id = $1")
            .bind(cat_id)
            .fetch_optional(&mut **tx)
            .await?;
        Ok(row.map(|r| category_type_from_db(&r.0)))
    }

    // ─────────────────────────────────────────────────────────────────
    // Public write methods
    // ─────────────────────────────────────────────────────────────────

    /// Create a brand-new logical transaction. Encrypts the amount and
    /// description with the session DEK, inserts a ledger row, seeds
    /// `logical_transaction_state`, and applies the balance side effect
    /// to the affected accounts — all atomically in one DB transaction.
    pub async fn create_transaction(&self, transaction: &TransactionRequest, user_id: &Uuid, dek: &Dek) -> Result<LedgerInsertResult, AppError> {
        self.validate_transaction_ownership(transaction, user_id).await?;

        let amount_enc = dek.encrypt_i64(transaction.amount)?;
        let description_enc = dek.encrypt_string(&transaction.description)?;

        let mut tx = self.pool.begin().await?;

        let cat_type = self.resolve_category_type(&mut tx, Some(&transaction.category_id)).await?;

        let (id, seq, created_at) = self
            .insert_ledger_row_enc_in_tx(
                &mut tx,
                None,
                user_id,
                &amount_enc,
                &description_enc,
                transaction.occurred_at,
                Some(&transaction.category_id),
                &transaction.from_account_id,
                transaction.to_account_id.as_ref(),
                transaction.vendor_id.as_ref(),
            )
            .await?;

        self.upsert_lts_in_tx(&mut tx, dek, &id, user_id, transaction.amount, seq, created_at).await?;

        self.apply_category_balance_effect(
            &mut tx,
            cat_type,
            transaction.amount,
            &transaction.from_account_id,
            transaction.to_account_id.as_ref(),
            dek,
        )
        .await?;

        tx.commit().await?;

        Ok(LedgerInsertResult {
            id,
            seq,
            first_created_at: created_at,
            occurred_at: transaction.occurred_at,
            from_account_id: transaction.from_account_id,
            to_account_id: transaction.to_account_id,
            category_id: Some(transaction.category_id),
            vendor_id: transaction.vendor_id,
            amount_enc,
            description_enc,
        })
    }

    /// Void (delete) a logical transaction by inserting a compensating
    /// row with `amount = -current_sum` under the same `id`. The row's
    /// metadata is copied from the Latest_Row so the balance side effect
    /// correctly undoes the prior accumulated delta.
    pub async fn delete_transaction(&self, id: &Uuid, user_id: &Uuid, dek: &Dek) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let state = self
            .lock_logical_transaction_state(&mut tx, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        if !state.is_effective {
            return Err(AppError::Conflict("Transaction has already been voided".to_string()));
        }

        let prev_sum = dek.decrypt_i64(&state.current_sum_enc)?;
        let latest = self.fetch_latest_row(&mut tx, id, state.latest_seq).await?;
        let cat_type = self.resolve_category_type(&mut tx, latest.category_id.as_ref()).await?;

        // Encrypt the compensating amount. Description bytes pass through
        // verbatim — we're preserving the Latest_Row's description on the
        // void compensating row (matches the ledger refactor's semantics).
        let compensating_amount = -prev_sum;
        let amount_enc = dek.encrypt_i64(compensating_amount)?;

        let (_, seq, _) = self
            .insert_ledger_row_enc_in_tx(
                &mut tx,
                Some(id),
                user_id,
                &amount_enc,
                &latest.description_enc,
                latest.occurred_at,
                latest.category_id.as_ref(),
                &latest.from_account_id,
                latest.to_account_id.as_ref(),
                latest.vendor_id.as_ref(),
            )
            .await?;

        self.upsert_lts_in_tx(&mut tx, dek, id, user_id, compensating_amount, seq, chrono::Utc::now())
            .await?;

        self.apply_category_balance_effect(
            &mut tx,
            cat_type,
            compensating_amount,
            &latest.from_account_id,
            latest.to_account_id.as_ref(),
            dek,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Correct (update) a logical transaction by inserting a reversal
    /// row (bringing the running sum to 0) followed by a correction row
    /// (applying the desired new metadata and amount). Both inserts
    /// happen atomically inside one DB transaction.
    pub async fn update_transaction(&self, id: &Uuid, transaction: &TransactionRequest, user_id: &Uuid, dek: &Dek) -> Result<LedgerInsertResult, AppError> {
        self.validate_transaction_ownership(transaction, user_id).await?;

        let amount_enc = dek.encrypt_i64(transaction.amount)?;
        let description_enc = dek.encrypt_string(&transaction.description)?;

        let mut tx = self.pool.begin().await?;

        let state = self
            .lock_logical_transaction_state(&mut tx, id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        if !state.is_effective {
            return Err(AppError::Conflict("Transaction has already been voided".to_string()));
        }

        let prev_sum = dek.decrypt_i64(&state.current_sum_enc)?;
        let latest = self.fetch_latest_row(&mut tx, id, state.latest_seq).await?;
        let old_cat_type = self.resolve_category_type(&mut tx, latest.category_id.as_ref()).await?;
        let new_cat_type = self.resolve_category_type(&mut tx, Some(&transaction.category_id)).await?;

        // Full_Reversal_Row: brings the running sum to zero. Copies all
        // metadata (including description_enc) from the Latest_Row.
        let reversal_amount = -prev_sum;
        let reversal_amount_enc = dek.encrypt_i64(reversal_amount)?;
        let (_, reversal_seq, _) = self
            .insert_ledger_row_enc_in_tx(
                &mut tx,
                Some(id),
                user_id,
                &reversal_amount_enc,
                &latest.description_enc,
                latest.occurred_at,
                latest.category_id.as_ref(),
                &latest.from_account_id,
                latest.to_account_id.as_ref(),
                latest.vendor_id.as_ref(),
            )
            .await?;

        self.upsert_lts_in_tx(&mut tx, dek, id, user_id, reversal_amount, reversal_seq, chrono::Utc::now())
            .await?;

        self.apply_category_balance_effect(
            &mut tx,
            old_cat_type,
            reversal_amount,
            &latest.from_account_id,
            latest.to_account_id.as_ref(),
            dek,
        )
        .await?;

        // Correction_Row: carries the new metadata + desired amount.
        let (_, correction_seq, correction_created_at) = self
            .insert_ledger_row_enc_in_tx(
                &mut tx,
                Some(id),
                user_id,
                &amount_enc,
                &description_enc,
                transaction.occurred_at,
                Some(&transaction.category_id),
                &transaction.from_account_id,
                transaction.to_account_id.as_ref(),
                transaction.vendor_id.as_ref(),
            )
            .await?;

        self.upsert_lts_in_tx(&mut tx, dek, id, user_id, transaction.amount, correction_seq, correction_created_at)
            .await?;

        self.apply_category_balance_effect(
            &mut tx,
            new_cat_type,
            transaction.amount,
            &transaction.from_account_id,
            transaction.to_account_id.as_ref(),
            dek,
        )
        .await?;

        tx.commit().await?;

        Ok(LedgerInsertResult {
            id: *id,
            seq: correction_seq,
            first_created_at: state.first_created_at,
            occurred_at: transaction.occurred_at,
            from_account_id: transaction.from_account_id,
            to_account_id: transaction.to_account_id,
            category_id: Some(transaction.category_id),
            vendor_id: transaction.vendor_id,
            amount_enc,
            description_enc,
        })
    }

    /// Create N logical transactions inside a single database transaction.
    /// All-or-nothing: any failure rolls the whole batch back.
    pub async fn batch_create_transactions(&self, transactions: &[TransactionRequest], user_id: &Uuid, dek: &Dek) -> Result<Vec<LedgerInsertResult>, AppError> {
        for req in transactions {
            self.validate_transaction_ownership(req, user_id).await?;
        }

        let mut tx = self.pool.begin().await?;
        let mut results = Vec::with_capacity(transactions.len());

        for req in transactions {
            let amount_enc = dek.encrypt_i64(req.amount)?;
            let description_enc = dek.encrypt_string(&req.description)?;
            let cat_type = self.resolve_category_type(&mut tx, Some(&req.category_id)).await?;

            let (id, seq, created_at) = self
                .insert_ledger_row_enc_in_tx(
                    &mut tx,
                    None,
                    user_id,
                    &amount_enc,
                    &description_enc,
                    req.occurred_at,
                    Some(&req.category_id),
                    &req.from_account_id,
                    req.to_account_id.as_ref(),
                    req.vendor_id.as_ref(),
                )
                .await?;

            self.upsert_lts_in_tx(&mut tx, dek, &id, user_id, req.amount, seq, created_at).await?;
            self.apply_category_balance_effect(&mut tx, cat_type, req.amount, &req.from_account_id, req.to_account_id.as_ref(), dek)
                .await?;

            results.push(LedgerInsertResult {
                id,
                seq,
                first_created_at: created_at,
                occurred_at: req.occurred_at,
                from_account_id: req.from_account_id,
                to_account_id: req.to_account_id,
                category_id: Some(req.category_id),
                vendor_id: req.vendor_id,
                amount_enc,
                description_enc,
            });
        }

        tx.commit().await?;
        Ok(results)
    }
}
