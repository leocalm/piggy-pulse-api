#![allow(dead_code)]
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::{ColorTheme, DashboardLayout, DateFormat, NumberFormat, Theme};
use crate::error::app_error::AppError;
use crate::models::settings::Settings;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct ExportTransactionRow {
    pub date: String,
    pub description: String,
    pub amount: i64,
    pub currency: String,
    pub category: String,
    pub tx_type: String,
    pub from_account: String,
    pub to_account: String,
    pub vendor: String,
}

// ── V2 helper types ──────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ProfileV2Row {
    name: String,
    currency: String,
    avatar: String,
}

#[derive(sqlx::FromRow)]
struct PreferencesV2Row {
    theme: String,
    date_format: String,
    number_format: String,
    language: String,
    compact_mode: bool,
    dashboard_layout: serde_json::Value,
    color_theme: String,
}

fn parse_theme(s: &str) -> Theme {
    match s {
        "dark" => Theme::Dark,
        "system" => Theme::System,
        _ => Theme::Light,
    }
}

fn parse_date_format(s: &str) -> DateFormat {
    match s {
        "MM/DD/YYYY" => DateFormat::MmDdYyyy,
        "YYYY-MM-DD" => DateFormat::YyyyMmDd,
        _ => DateFormat::DdMmYyyy,
    }
}

fn parse_number_format(s: &str) -> NumberFormat {
    match s {
        "1.234,56" => NumberFormat::PeriodComma,
        "1 234,56" => NumberFormat::SpaceComma,
        _ => NumberFormat::CommaPeriod,
    }
}

// IMPORTANT: `parse_color_theme` and `color_theme_str` must stay in sync with the
// CHECK constraint defined in migration 20260327000002_add_color_theme_to_settings.
fn parse_color_theme(s: &str) -> ColorTheme {
    match s {
        "sunrise" => ColorTheme::Sunrise,
        "sage_stone" => ColorTheme::SageStone,
        "deep_ocean" => ColorTheme::DeepOcean,
        "warm_rose" => ColorTheme::WarmRose,
        "moonlit" => ColorTheme::Moonlit,
        "nebula" => ColorTheme::Nebula,
        _ => {
            tracing::warn!(value = s, "Unknown color_theme value in database; falling back to Nebula");
            ColorTheme::Nebula
        }
    }
}

// IMPORTANT: must stay in sync with the CHECK constraint in migration 20260327000002.
fn color_theme_str(ct: ColorTheme) -> &'static str {
    match ct {
        ColorTheme::Nebula => "nebula",
        ColorTheme::Sunrise => "sunrise",
        ColorTheme::SageStone => "sage_stone",
        ColorTheme::DeepOcean => "deep_ocean",
        ColorTheme::WarmRose => "warm_rose",
        ColorTheme::Moonlit => "moonlit",
    }
}

impl PostgresRepository {
    pub async fn get_settings(&self, user_id: &Uuid) -> Result<Settings, AppError> {
        let settings = sqlx::query_as::<_, Settings>(
            r#"
            SELECT id, user_id, theme, language, default_currency_id,
                   budget_stability_tolerance_basis_points,
                   created_at, updated_at
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(settings)
    }

    /// Update only the default currency on an existing settings row.
    #[allow(dead_code)]
    pub async fn update_settings_currency(&self, user_id: &Uuid, currency_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE settings SET default_currency_id = $1, updated_at = now() WHERE user_id = $2")
            .bind(currency_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_default_settings(&self, user_id: &Uuid) -> Result<Settings, AppError> {
        let settings = sqlx::query_as::<_, Settings>(
            r#"
            INSERT INTO settings (
                user_id,
                theme,
                language,
                default_currency_id,
                budget_stability_tolerance_basis_points
            )
            VALUES ($1, 'light', 'en', NULL, 1000)
            RETURNING id, user_id, theme, language, default_currency_id,
                      budget_stability_tolerance_basis_points,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(settings)
    }

    // ── V2 Profile ─────────────────────────────────────────────────────────

    pub async fn get_profile_v2(&self, user_id: &Uuid) -> Result<crate::dto::settings::ProfileResponse, AppError> {
        let row = sqlx::query_as::<_, ProfileV2Row>(
            r#"
            SELECT u.name,
                   COALESCE(c.currency, '') AS currency,
                   u.avatar
            FROM users u
            LEFT JOIN settings s ON s.user_id = u.id
            LEFT JOIN currency c ON c.id = s.default_currency_id
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::dto::settings::ProfileResponse {
            name: row.name,
            currency: row.currency,
            avatar: row.avatar,
        })
    }

    pub async fn update_profile_v2(
        &self,
        user_id: &Uuid,
        name: &str,
        currency_code: &str,
        avatar: Option<&str>,
    ) -> Result<crate::dto::settings::ProfileResponse, AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE users SET name = $1, avatar = COALESCE($2, avatar) WHERE id = $3")
            .bind(name)
            .bind(avatar)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Resolve currency code to id
        let currency_id: Option<Uuid> = if currency_code.is_empty() {
            None
        } else {
            let id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM currency WHERE currency = $1 LIMIT 1")
                .bind(currency_code)
                .fetch_optional(&mut *tx)
                .await?;

            if id.is_none() {
                return Err(AppError::BadRequest(format!("Currency '{}' not found", currency_code)));
            }
            id
        };

        sqlx::query("UPDATE settings SET default_currency_id = $1, updated_at = now() WHERE user_id = $2")
            .bind(currency_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        self.get_profile_v2(user_id).await
    }

    // ── V2 Preferences ───────────────────────────────────────────────────────

    pub async fn get_preferences_v2(&self, user_id: &Uuid) -> Result<crate::dto::settings::PreferencesResponse, AppError> {
        let row = sqlx::query_as::<_, PreferencesV2Row>(
            r#"
            SELECT theme, date_format, number_format, language, compact_mode, dashboard_layout, color_theme
            FROM settings
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let dashboard_layout: DashboardLayout = serde_json::from_value(row.dashboard_layout).unwrap_or_default();

        Ok(crate::dto::settings::PreferencesResponse {
            theme: parse_theme(&row.theme),
            date_format: parse_date_format(&row.date_format),
            number_format: parse_number_format(&row.number_format),
            language: row.language,
            compact_mode: row.compact_mode,
            dashboard_layout,
            color_theme: parse_color_theme(&row.color_theme),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_preferences_v2(
        &self,
        user_id: &Uuid,
        theme: &str,
        date_format: &str,
        number_format: &str,
        language: &str,
        compact_mode: bool,
        dashboard_layout: &DashboardLayout,
        color_theme: ColorTheme,
    ) -> Result<crate::dto::settings::PreferencesResponse, AppError> {
        let layout_json = serde_json::to_value(dashboard_layout).unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE settings
            SET theme = $1, date_format = $2, number_format = $3, language = $4,
                compact_mode = $5, dashboard_layout = $6, color_theme = $7, updated_at = now()
            WHERE user_id = $8
            "#,
        )
        .bind(theme)
        .bind(date_format)
        .bind(number_format)
        .bind(language)
        .bind(compact_mode)
        .bind(layout_json)
        .bind(color_theme_str(color_theme))
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        self.get_preferences_v2(user_id).await
    }

    // ── V2 Export (retired) ───────────────────────────────────────────────────
    //
    // Export and import are retired under encryption-at-rest. The
    // server no longer has plaintext access to transaction amounts,
    // descriptions, or entity names, so it cannot produce a CSV or
    // JSON dump. The client exports from its own decrypted view.
    pub async fn export_transactions_v2(&self, user_id: &Uuid) -> Result<Vec<ExportTransactionRow>, AppError> {
        let rows = sqlx::query_as::<_, ExportTransactionRow>(
            r#"
            SELECT t.occurred_at::text AS date,
                   t.description,
                   t.amount,
                   COALESCE(cur.currency, '') AS currency,
                   COALESCE(cat.name, '') AS category,
                   CASE
                       WHEN t.to_account_id IS NOT NULL THEN 'transfer'
                       WHEN cat.category_type = 'Incoming' THEN 'incoming'
                       ELSE 'outgoing'
                   END AS tx_type,
                   COALESCE(fa.name, '') AS from_account,
                   COALESCE(ta.name, '') AS to_account,
                   COALESCE(v.name, '') AS vendor
            FROM transaction t
            LEFT JOIN account fa ON fa.id = t.from_account_id
            LEFT JOIN account ta ON ta.id = t.to_account_id
            LEFT JOIN currency cur ON cur.id = fa.currency_id
            LEFT JOIN category cat ON cat.id = t.category_id
            LEFT JOIN vendor v ON v.id = t.vendor_id
            WHERE t.user_id = $1
            ORDER BY t.occurred_at DESC, t.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn export_all_data_v2(&self, user_id: &Uuid) -> Result<serde_json::Value, AppError> {
        let accounts = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT COALESCE(json_agg(row_to_json(a)), '[]'::json) FROM (
                SELECT id, name, account_type, color, is_archived, currency_id, created_at
                FROM account WHERE user_id = $1 ORDER BY created_at
            ) a
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(serde_json::Value::Array(vec![]));

        let categories = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT COALESCE(json_agg(row_to_json(c)), '[]'::json) FROM (
                SELECT id, name, category_type, color, icon, is_system, created_at
                FROM category WHERE user_id = $1 ORDER BY created_at
            ) c
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(serde_json::Value::Array(vec![]));

        let transactions = sqlx::query_scalar::<_, serde_json::Value>(
            r#"
            SELECT COALESCE(json_agg(row_to_json(t)), '[]'::json) FROM (
                SELECT id, description, amount, occurred_at, from_account_id, to_account_id, category_id, vendor_id, created_at
                FROM transaction WHERE user_id = $1 ORDER BY occurred_at DESC
            ) t
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(serde_json::Value::Array(vec![]));

        Ok(serde_json::json!({
            "accounts": accounts,
            "categories": categories,
            "transactions": transactions,
        }))
    }

    /// Import data from a JSON backup (reverse of `export_all_data_v2`).
    ///
    /// Inserts accounts, categories, and transactions from the provided JSON blob.
    /// Fresh UUIDs are generated for every imported row so that a backup from
    /// one account can be safely imported into a different account without ID
    /// collisions. Old→new ID mappings are maintained in memory so that
    /// transaction foreign-key references are correctly remapped.
    ///
    /// Returns the counts of imported rows: `(accounts, categories, transactions)`.
    pub async fn import_data_v2(&self, user_id: &Uuid, data: &serde_json::Value) -> Result<(usize, usize, usize), AppError> {
        use std::collections::HashMap;

        let empty_vec = serde_json::Value::Array(vec![]);

        let accounts_json = data.get("accounts").unwrap_or(&empty_vec);
        let categories_json = data.get("categories").unwrap_or(&empty_vec);
        let transactions_json = data.get("transactions").unwrap_or(&empty_vec);

        let accounts_arr = accounts_json
            .as_array()
            .ok_or_else(|| AppError::BadRequest("'accounts' must be an array".to_string()))?;
        let categories_arr = categories_json
            .as_array()
            .ok_or_else(|| AppError::BadRequest("'categories' must be an array".to_string()))?;
        let transactions_arr = transactions_json
            .as_array()
            .ok_or_else(|| AppError::BadRequest("'transactions' must be an array".to_string()))?;

        let mut tx = self.pool.begin().await?;

        // ── Accounts ─────────────────────────────────────────────────────────
        let mut account_id_map: HashMap<String, Uuid> = HashMap::new();

        for acct in accounts_arr {
            let old_id = acct
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("account missing 'id'".to_string()))?;
            let name = acct
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("account missing 'name'".to_string()))?;
            let account_type = acct
                .get("account_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("account missing 'account_type'".to_string()))?;
            let color = acct.get("color").and_then(|v| v.as_str()).unwrap_or("#868E96");
            let is_archived = acct.get("is_archived").and_then(|v| v.as_bool()).unwrap_or(false);
            let currency_id = acct.get("currency_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());

            let new_id = Uuid::new_v4();
            account_id_map.insert(old_id.to_string(), new_id);

            sqlx::query(
                r#"
                INSERT INTO account (id, user_id, name, account_type, color, icon, balance, is_archived, currency_id)
                VALUES ($1, $2, $3, $4::text::account_type, $5, '', 0, $6, $7)
                "#,
            )
            .bind(new_id)
            .bind(user_id)
            .bind(name)
            .bind(account_type)
            .bind(color)
            .bind(is_archived)
            .bind(currency_id)
            .execute(&mut *tx)
            .await?;
        }

        // ── Categories ───────────────────────────────────────────────────────
        let mut category_id_map: HashMap<String, Uuid> = HashMap::new();

        for cat in categories_arr {
            let old_id = cat
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("category missing 'id'".to_string()))?;
            let name = cat
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("category missing 'name'".to_string()))?;
            let category_type = cat
                .get("category_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("category missing 'category_type'".to_string()))?;
            let color = cat.get("color").and_then(|v| v.as_str()).unwrap_or("#868E96");
            let icon = cat.get("icon").and_then(|v| v.as_str()).unwrap_or("?");
            let is_system = cat.get("is_system").and_then(|v| v.as_bool()).unwrap_or(false);

            // System categories (e.g. Transfer) may already exist after reset-structure.
            // Map the old ID to the existing system category instead of inserting a duplicate.
            if is_system {
                let existing: Option<(Uuid,)> =
                    sqlx::query_as("SELECT id FROM category WHERE user_id = $1 AND name = $2 AND category_type = $3::text::category_type AND is_system = true")
                        .bind(user_id)
                        .bind(name)
                        .bind(category_type)
                        .fetch_optional(&mut *tx)
                        .await?;
                if let Some((existing_id,)) = existing {
                    category_id_map.insert(old_id.to_string(), existing_id);
                    continue;
                }
            }

            let new_id = Uuid::new_v4();
            category_id_map.insert(old_id.to_string(), new_id);

            sqlx::query(
                r#"
                INSERT INTO category (id, user_id, name, category_type, color, icon, is_system)
                VALUES ($1, $2, $3, $4::text::category_type, $5, $6, $7)
                "#,
            )
            .bind(new_id)
            .bind(user_id)
            .bind(name)
            .bind(category_type)
            .bind(color)
            .bind(icon)
            .bind(is_system)
            .execute(&mut *tx)
            .await?;
        }

        // ── Transactions ─────────────────────────────────────────────────────
        let mut imported_transactions: usize = 0;

        for txn in transactions_arr {
            let description = txn.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let amount = txn
                .get("amount")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| AppError::BadRequest("transaction missing 'amount'".to_string()))?;
            let occurred_at = txn
                .get("occurred_at")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("transaction missing 'occurred_at'".to_string()))?;

            let from_account_old_id = txn
                .get("from_account_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("transaction missing 'from_account_id'".to_string()))?;
            let from_account_id = account_id_map
                .get(from_account_old_id)
                .copied()
                .ok_or_else(|| AppError::BadRequest(format!("transaction references unknown from_account_id '{}'", from_account_old_id)))?;

            let to_account_id = txn
                .get("to_account_id")
                .and_then(|v| if v.is_null() { None } else { v.as_str() })
                .and_then(|old| account_id_map.get(old).copied());

            let category_old_id = txn
                .get("category_id")
                .and_then(|v| if v.is_null() { None } else { v.as_str() })
                .ok_or_else(|| AppError::BadRequest("transaction missing 'category_id'".to_string()))?;
            let category_id = category_id_map
                .get(category_old_id)
                .copied()
                .ok_or_else(|| AppError::BadRequest(format!("transaction references unknown category_id '{}'", category_old_id)))?;

            let new_id = Uuid::new_v4();

            sqlx::query(
                r#"
                INSERT INTO transaction (id, user_id, description, amount, occurred_at, from_account_id, to_account_id, category_id)
                VALUES ($1, $2, $3, $4, $5::date, $6, $7, $8)
                "#,
            )
            .bind(new_id)
            .bind(user_id)
            .bind(description)
            .bind(amount)
            .bind(occurred_at)
            .bind(from_account_id)
            .bind(to_account_id)
            .bind(category_id)
            .execute(&mut *tx)
            .await?;

            imported_transactions += 1;
        }

        tx.commit().await?;

        Ok((accounts_arr.len(), categories_arr.len(), imported_transactions))
    }

    // ── V2 Reset Structure ───────────────────────────────────────────────────

    /// V2 reset: also deletes vendors (unlike V1)
    pub async fn reset_structure_v2(&self, user_id: &Uuid) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // Bulk reset legitimately cascade-deletes transactions via the
        // account/category/vendor FKs. Enable the ledger mutation bypass for
        // this transaction only; it is cleared automatically on commit.
        sqlx::query("SET LOCAL piggy_pulse.allow_ledger_mutations = 'on'").execute(&mut *tx).await?;

        sqlx::query("DELETE FROM period_schedule WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM budget_period WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM account WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM category WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        sqlx::query("DELETE FROM vendor WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        // Re-create the system Transfer category
        sqlx::query(
            r#"
            INSERT INTO category (user_id, name, color, icon, category_type, is_system)
            VALUES ($1, 'Transfer', '#868E96', '↔', 'Transfer'::category_type, TRUE)
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Deletes all user data (for account deletion). Unlike reset_structure,
    /// this does NOT re-create the system Transfer category.
    pub async fn delete_all_user_data(&self, user_id: &Uuid) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // Same rationale as reset_structure_v2: the cascade chain will remove
        // transactions; bypass the ledger immutability trigger for this tx only.
        sqlx::query("SET LOCAL piggy_pulse.allow_ledger_mutations = 'on'").execute(&mut *tx).await?;

        sqlx::query("DELETE FROM period_schedule WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM budget_period WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM account WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM category WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM vendor WHERE user_id = $1").bind(user_id).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}
