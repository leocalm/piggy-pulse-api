use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::account::AccountResponse;
use crate::models::category::CategoryResponse;
use crate::models::overlay::{
    InclusionMode, InclusionSource, Overlay, OverlayCategoryCap, OverlayRequest, OverlayRules, OverlayWithMetrics, TransactionMembership,
    TransactionWithMembership,
};
use crate::models::transaction::TransactionResponse;
use crate::models::vendor::VendorResponse;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

// Helper struct to group a few transaction fields so we don't exceed the function
// parameter limit when checking overlay inclusion rules.
struct SimpleTransactionRef<'a> {
    id: &'a Uuid,
    category_id: &'a Uuid,
    from_account_id: &'a Uuid,
    vendor_id: &'a Option<Uuid>,
}

impl PostgresRepository {
    // ===== Create Overlay =====

    pub async fn create_overlay(&self, request: &OverlayRequest, user_id: &Uuid) -> Result<OverlayWithMetrics, AppError> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // Insert overlay
        #[derive(sqlx::FromRow)]
        struct OverlayRow {
            id: Uuid,
        }

        let overlay_row = sqlx::query_as::<_, OverlayRow>(
            r#"
            INSERT INTO overlays (user_id, name, icon, start_date, end_date, inclusion_mode, total_cap_amount, rules)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(&request.name)
        .bind(&request.icon)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(request.inclusion_mode)
        .bind(request.total_cap_amount)
        .bind(sqlx::types::Json(&request.rules))
        .fetch_one(&mut *tx)
        .await?;

        // Insert category caps
        for cap in &request.category_caps {
            sqlx::query(
                r#"
                INSERT INTO overlay_category_caps (overlay_id, category_id, cap_amount)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(overlay_row.id)
            .bind(cap.category_id)
            .bind(cap.cap_amount)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Fetch the overlay with metrics
        self.get_overlay(&overlay_row.id, user_id).await
    }

    // ===== Get Overlay =====

    pub async fn get_overlay(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<OverlayWithMetrics, AppError> {
        #[derive(sqlx::FromRow)]
        struct OverlayRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            icon: Option<String>,
            start_date: NaiveDate,
            end_date: NaiveDate,
            inclusion_mode: InclusionMode,
            total_cap_amount: Option<i64>,
            rules: sqlx::types::Json<OverlayRules>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let overlay_row = sqlx::query_as::<_, OverlayRow>(
            r#"
            SELECT id, user_id, name, icon, start_date, end_date, inclusion_mode, total_cap_amount, rules, created_at, updated_at
            FROM overlays
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(overlay_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Fetch category caps
        #[derive(sqlx::FromRow)]
        struct CapRow {
            category_id: Uuid,
            cap_amount: i64,
        }

        let cap_rows = sqlx::query_as::<_, CapRow>(
            r#"
            SELECT category_id, cap_amount
            FROM overlay_category_caps
            WHERE overlay_id = $1
            "#,
        )
        .bind(overlay_id)
        .fetch_all(&self.pool)
        .await?;

        let category_caps: Vec<OverlayCategoryCap> = cap_rows
            .iter()
            .map(|row| OverlayCategoryCap {
                category_id: row.category_id,
                cap_amount: row.cap_amount,
            })
            .collect();

        // Calculate spent amount and transaction count
        let (spent_amount, transaction_count) = self
            .calculate_overlay_metrics(
                overlay_id,
                &overlay_row.inclusion_mode,
                &overlay_row.start_date,
                &overlay_row.end_date,
                &overlay_row.rules.0,
                user_id,
            )
            .await?;

        Ok(OverlayWithMetrics {
            overlay: Overlay {
                id: overlay_row.id,
                user_id: overlay_row.user_id,
                name: overlay_row.name,
                icon: overlay_row.icon,
                start_date: overlay_row.start_date,
                end_date: overlay_row.end_date,
                inclusion_mode: overlay_row.inclusion_mode,
                total_cap_amount: overlay_row.total_cap_amount,
                rules: overlay_row.rules.0,
                created_at: overlay_row.created_at,
                updated_at: overlay_row.updated_at,
            },
            spent_amount,
            transaction_count,
            category_caps,
        })
    }

    // ===== List Overlays =====

    pub async fn list_overlays(&self, user_id: &Uuid) -> Result<Vec<OverlayWithMetrics>, AppError> {
        #[derive(sqlx::FromRow)]
        struct OverlayRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            icon: Option<String>,
            start_date: NaiveDate,
            end_date: NaiveDate,
            inclusion_mode: InclusionMode,
            total_cap_amount: Option<i64>,
            rules: sqlx::types::Json<OverlayRules>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let overlay_rows = sqlx::query_as::<_, OverlayRow>(
            r#"
            SELECT id, user_id, name, icon, start_date, end_date, inclusion_mode, total_cap_amount, rules, created_at, updated_at
            FROM overlays
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut overlays_with_metrics = Vec::new();

        for overlay_row in overlay_rows {
            #[derive(sqlx::FromRow)]
            struct CapRow {
                category_id: Uuid,
                cap_amount: i64,
            }

            let cap_rows = sqlx::query_as::<_, CapRow>(
                r#"
                SELECT category_id, cap_amount
                FROM overlay_category_caps
                WHERE overlay_id = $1
                "#,
            )
            .bind(overlay_row.id)
            .fetch_all(&self.pool)
            .await?;

            let category_caps: Vec<OverlayCategoryCap> = cap_rows
                .iter()
                .map(|row| OverlayCategoryCap {
                    category_id: row.category_id,
                    cap_amount: row.cap_amount,
                })
                .collect();

            let (spent_amount, transaction_count) = self
                .calculate_overlay_metrics(
                    &overlay_row.id,
                    &overlay_row.inclusion_mode,
                    &overlay_row.start_date,
                    &overlay_row.end_date,
                    &overlay_row.rules.0,
                    user_id,
                )
                .await?;

            overlays_with_metrics.push(OverlayWithMetrics {
                overlay: Overlay {
                    id: overlay_row.id,
                    user_id: overlay_row.user_id,
                    name: overlay_row.name.clone(),
                    icon: overlay_row.icon.clone(),
                    start_date: overlay_row.start_date,
                    end_date: overlay_row.end_date,
                    inclusion_mode: overlay_row.inclusion_mode,
                    total_cap_amount: overlay_row.total_cap_amount,
                    rules: overlay_row.rules.0.clone(),
                    created_at: overlay_row.created_at,
                    updated_at: overlay_row.updated_at,
                },
                spent_amount,
                transaction_count,
                category_caps,
            });
        }

        Ok(overlays_with_metrics)
    }

    // ===== Update Overlay =====

    pub async fn update_overlay(&self, overlay_id: &Uuid, request: &OverlayRequest, user_id: &Uuid) -> Result<OverlayWithMetrics, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query_as::<_, ()>(
            r#"
            UPDATE overlays
            SET name = $1, icon = $2, start_date = $3, end_date = $4,
                inclusion_mode = $5, total_cap_amount = $6, rules = $7, updated_at = now()
            WHERE id = $8 AND user_id = $9
            RETURNING id
            "#,
        )
        .bind(&request.name)
        .bind(&request.icon)
        .bind(request.start_date)
        .bind(request.end_date)
        .bind(request.inclusion_mode)
        .bind(request.total_cap_amount)
        .bind(sqlx::types::Json(&request.rules))
        .bind(overlay_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Delete existing category caps
        sqlx::query("DELETE FROM overlay_category_caps WHERE overlay_id = $1")
            .bind(overlay_id)
            .execute(&mut *tx)
            .await?;

        // Insert new category caps
        for cap in &request.category_caps {
            sqlx::query(
                r#"
                INSERT INTO overlay_category_caps (overlay_id, category_id, cap_amount)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(overlay_id)
            .bind(cap.category_id)
            .bind(cap.cap_amount)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Fetch the overlay with metrics
        self.get_overlay(overlay_id, user_id).await
    }

    // ===== Delete Overlay =====

    pub async fn delete_overlay(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM overlays WHERE id = $1 AND user_id = $2")
            .bind(overlay_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Get Overlay Transactions with Membership =====

    pub async fn get_overlay_transactions(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<Vec<TransactionWithMembership>, AppError> {
        // First, get the overlay to determine inclusion mode and rules
        #[derive(sqlx::FromRow)]
        struct OverlayInfo {
            inclusion_mode: InclusionMode,
            start_date: NaiveDate,
            end_date: NaiveDate,
            rules: sqlx::types::Json<OverlayRules>,
        }

        let overlay_info = sqlx::query_as::<_, OverlayInfo>(
            r#"
            SELECT inclusion_mode, start_date, end_date, rules
            FROM overlays
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(overlay_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Get all transactions in the date range with full details
        let transactions = self
            .list_transactions_in_date_range(user_id, &overlay_info.start_date, &overlay_info.end_date)
            .await?;

        // Get manual inclusions/exclusions
        #[derive(sqlx::FromRow)]
        struct InclusionRow {
            transaction_id: Uuid,
            is_included: bool,
        }

        let inclusion_rows = sqlx::query_as::<_, InclusionRow>(
            r#"
            SELECT transaction_id, is_included
            FROM overlay_transaction_inclusions
            WHERE overlay_id = $1
            "#,
        )
        .bind(overlay_id)
        .fetch_all(&self.pool)
        .await?;

        let manual_map: std::collections::HashMap<Uuid, bool> = inclusion_rows.iter().map(|row| (row.transaction_id, row.is_included)).collect();

        // Build transactions with membership
        let mut result = Vec::new();

        for tx in transactions {
            let (is_included, inclusion_source) =
                self.determine_transaction_membership(&tx.id, &overlay_info.inclusion_mode, &overlay_info.rules.0, &tx, &manual_map);

            result.push(TransactionWithMembership {
                transaction: tx,
                membership: TransactionMembership { is_included, inclusion_source },
            });
        }

        Ok(result)
    }

    // ===== Manual Include Transaction =====

    pub async fn include_transaction(&self, overlay_id: &Uuid, transaction_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Verify overlay belongs to user
        let _ = self.get_overlay(overlay_id, user_id).await?;

        sqlx::query(
            r#"
            INSERT INTO overlay_transaction_inclusions (overlay_id, transaction_id, is_included)
            VALUES ($1, $2, true)
            ON CONFLICT (overlay_id, transaction_id)
            DO UPDATE SET is_included = true
            "#,
        )
        .bind(overlay_id)
        .bind(transaction_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ===== Manual Exclude Transaction =====

    pub async fn exclude_transaction(&self, overlay_id: &Uuid, transaction_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Verify overlay belongs to user
        let _ = self.get_overlay(overlay_id, user_id).await?;

        sqlx::query("DELETE FROM overlay_transaction_inclusions WHERE overlay_id = $1 AND transaction_id = $2")
            .bind(overlay_id)
            .bind(transaction_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Helper Methods =====

    async fn calculate_overlay_metrics(
        &self,
        overlay_id: &Uuid,
        inclusion_mode: &InclusionMode,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
        rules: &OverlayRules,
        user_id: &Uuid,
    ) -> Result<(i64, i64), AppError> {
        // Get manual inclusions/exclusions
        #[derive(sqlx::FromRow)]
        struct InclusionRow {
            transaction_id: Uuid,
            is_included: bool,
        }

        let inclusion_rows = sqlx::query_as::<_, InclusionRow>(
            r#"
            SELECT transaction_id, is_included
            FROM overlay_transaction_inclusions
            WHERE overlay_id = $1
            "#,
        )
        .bind(overlay_id)
        .fetch_all(&self.pool)
        .await?;

        let manual_map: std::collections::HashMap<Uuid, bool> = inclusion_rows.iter().map(|row| (row.transaction_id, row.is_included)).collect();

        // Get transactions in date range
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            amount: i64,
            category_id: Uuid,
            from_account_id: Uuid,
            vendor_id: Option<Uuid>,
        }

        let transactions = sqlx::query_as::<_, TransactionRow>(
            r#"
            SELECT id, amount, category_id, from_account_id, vendor_id
            FROM transaction
            WHERE user_id = $1
                AND occurred_at >= $2
                AND occurred_at <= $3
            "#,
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut spent_amount = 0i64;
        let mut transaction_count = 0i64;

        for tx in transactions {
            // Build a small reference struct to avoid passing many parameters
            let simple_tx = SimpleTransactionRef {
                id: &tx.id,
                category_id: &tx.category_id,
                from_account_id: &tx.from_account_id,
                vendor_id: &tx.vendor_id,
            };

            let (is_included, _) = self.determine_transaction_membership_simple(&simple_tx, inclusion_mode, rules, &manual_map);

            if is_included {
                spent_amount += tx.amount;
                transaction_count += 1;
            }
        }

        Ok((spent_amount, transaction_count))
    }

    fn determine_transaction_membership(
        &self,
        transaction_id: &Uuid,
        inclusion_mode: &InclusionMode,
        rules: &OverlayRules,
        tx: &TransactionResponse,
        manual_map: &std::collections::HashMap<Uuid, bool>,
    ) -> (bool, Option<InclusionSource>) {
        // Check manual override first
        if let Some(&is_manually_included) = manual_map.get(transaction_id) {
            if is_manually_included {
                return (true, Some(InclusionSource::Manual));
            } else {
                return (false, None);
            }
        }

        // Apply inclusion mode logic
        match inclusion_mode {
            InclusionMode::Manual => (false, None),
            InclusionMode::All => (true, Some(InclusionSource::All)),
            InclusionMode::Rules => {
                let matches_rules = self.transaction_matches_rules_response(tx, rules);
                if matches_rules { (true, Some(InclusionSource::Rules)) } else { (false, None) }
            }
        }
    }

    fn determine_transaction_membership_simple(
        &self,
        tx: &SimpleTransactionRef,
        inclusion_mode: &InclusionMode,
        rules: &OverlayRules,
        manual_map: &std::collections::HashMap<Uuid, bool>,
    ) -> (bool, Option<InclusionSource>) {
        // Check manual override first
        if let Some(&is_manually_included) = manual_map.get(tx.id) {
            if is_manually_included {
                return (true, Some(InclusionSource::Manual));
            } else {
                return (false, None);
            }
        }

        // Apply inclusion mode logic
        match inclusion_mode {
            InclusionMode::Manual => (false, None),
            InclusionMode::All => (true, Some(InclusionSource::All)),
            InclusionMode::Rules => {
                let matches_rules = self.transaction_matches_rules_simple(tx.category_id, tx.from_account_id, tx.vendor_id, rules);
                if matches_rules { (true, Some(InclusionSource::Rules)) } else { (false, None) }
            }
        }
    }

    fn transaction_matches_rules_response(&self, tx: &TransactionResponse, rules: &OverlayRules) -> bool {
        let mut matches = false;

        // Check category
        if !rules.category_ids.is_empty() && rules.category_ids.contains(&tx.category.id) {
            matches = true;
        }

        // Check vendor
        if !rules.vendor_ids.is_empty()
            && let Some(ref vendor) = tx.vendor
            && rules.vendor_ids.contains(&vendor.id)
        {
            matches = true;
        }

        // Check account
        if !rules.account_ids.is_empty() && rules.account_ids.contains(&tx.from_account.id) {
            matches = true;
        }

        matches
    }

    fn transaction_matches_rules_simple(&self, category_id: &Uuid, from_account_id: &Uuid, vendor_id: &Option<Uuid>, rules: &OverlayRules) -> bool {
        let mut matches = false;

        // Check category
        if !rules.category_ids.is_empty() && rules.category_ids.contains(category_id) {
            matches = true;
        }

        // Check vendor
        if !rules.vendor_ids.is_empty()
            && let Some(v_id) = vendor_id
            && rules.vendor_ids.contains(v_id)
        {
            matches = true;
        }

        // Check account
        if !rules.account_ids.is_empty() && rules.account_ids.contains(from_account_id) {
            matches = true;
        }

        matches
    }

    // Helper to get transactions in date range with full details
    async fn list_transactions_in_date_range(
        &self,
        user_id: &Uuid,
        start_date: &NaiveDate,
        end_date: &NaiveDate,
    ) -> Result<Vec<TransactionResponse>, AppError> {
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            amount: i64,
            description: String,
            occurred_at: NaiveDate,
            category_id: Uuid,
            from_account_id: Uuid,
            to_account_id: Option<Uuid>,
            vendor_id: Option<Uuid>,
        }

        let transaction_rows = sqlx::query_as::<_, TransactionRow>(
            r#"
            SELECT id, amount, description, occurred_at, category_id, from_account_id, to_account_id, vendor_id
            FROM transaction
            WHERE user_id = $1
                AND occurred_at >= $2
                AND occurred_at <= $3
            ORDER BY occurred_at DESC, id DESC
            "#,
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();

        for tx_row in transaction_rows {
            let category_option = self.get_category_by_id(&tx_row.category_id, user_id).await?;
            let category = category_option.ok_or_else(|| AppError::NotFound(format!("Category {} not found", tx_row.category_id)))?;
            let from_account = self.get_account_by_id_simple(&tx_row.from_account_id, user_id).await?;
            let to_account = if let Some(to_id) = tx_row.to_account_id {
                Some(self.get_account_by_id_simple(&to_id, user_id).await?)
            } else {
                None
            };
            let vendor = if let Some(vendor_id) = tx_row.vendor_id {
                Some(self.get_vendor_by_id_simple(&vendor_id, user_id).await?)
            } else {
                None
            };

            result.push(TransactionResponse {
                id: tx_row.id,
                amount: tx_row.amount,
                description: tx_row.description,
                occurred_at: tx_row.occurred_at,
                category: CategoryResponse::from(&category),
                from_account: AccountResponse::from(&from_account),
                to_account: to_account.as_ref().map(AccountResponse::from),
                vendor: vendor.as_ref().map(VendorResponse::from),
            });
        }

        Ok(result)
    }

    // Simple account fetcher (without full metrics)
    async fn get_account_by_id_simple(&self, account_id: &Uuid, user_id: &Uuid) -> Result<crate::models::account::Account, AppError> {
        use crate::database::account::account_type_from_db;

        #[derive(sqlx::FromRow)]
        struct AccountRow {
            id: Uuid,
            user_id: Uuid,
            name: String,
            color: String,
            icon: String,
            account_type: String,
            currency_id: Uuid,
            spend_limit: Option<i32>,
            created_at: DateTime<Utc>,
        }

        let account_row = sqlx::query_as::<_, AccountRow>(
            r#"
            SELECT id, user_id, name, color, icon, account_type, currency_id, spend_limit, created_at
            FROM account
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(account_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Fetch currency
        let currency = sqlx::query_as::<_, crate::models::currency::Currency>(
            r#"
            SELECT id, name, symbol, currency, decimal_places, created_at
            FROM currency
            WHERE id = $1
            "#,
        )
        .bind(account_row.currency_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::models::account::Account {
            id: account_row.id,
            user_id: account_row.user_id,
            name: account_row.name,
            color: account_row.color,
            icon: account_row.icon,
            account_type: account_type_from_db(&account_row.account_type),
            currency,
            balance: 0, // Not needed for overlay context
            spend_limit: account_row.spend_limit,
            created_at: account_row.created_at,
        })
    }

    // Simple vendor fetcher
    async fn get_vendor_by_id_simple(&self, vendor_id: &Uuid, user_id: &Uuid) -> Result<crate::models::vendor::Vendor, AppError> {
        let vendor = sqlx::query_as::<_, crate::models::vendor::Vendor>(
            r#"
            SELECT id, user_id, name, created_at
            FROM vendor
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(vendor_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(vendor)
    }
}
