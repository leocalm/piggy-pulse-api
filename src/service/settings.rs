#![allow(dead_code)]
use uuid::Uuid;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::settings::{
    DateFormat, NumberFormat, PreferencesResponse, ProfileResponse, SessionResponse, Theme, UpdatePreferencesRequest, UpdateProfileRequest,
};
use crate::error::app_error::AppError;

pub struct SettingsService<'a> {
    pub repository: &'a PostgresRepository,
}

impl<'a> SettingsService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        Self { repository }
    }

    // ── Profile ──────────────────────────────────────────────────────────────

    pub async fn get_profile(&self, user_id: &Uuid) -> Result<ProfileResponse, AppError> {
        let row = self.repository.get_profile_v2(user_id).await?;
        Ok(row)
    }

    pub async fn update_profile(&self, user_id: &Uuid, request: &UpdateProfileRequest) -> Result<ProfileResponse, AppError> {
        self.repository
            .update_profile_v2(user_id, &request.name, &request.currency, &request.avatar)
            .await
    }

    // ── Preferences ──────────────────────────────────────────────────────────

    pub async fn get_preferences(&self, user_id: &Uuid) -> Result<PreferencesResponse, AppError> {
        self.repository.get_preferences_v2(user_id).await
    }

    pub async fn update_preferences(&self, user_id: &Uuid, request: &UpdatePreferencesRequest) -> Result<PreferencesResponse, AppError> {
        let theme_str = match request.theme {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => "system",
        };
        let date_format_str = match request.date_format {
            DateFormat::DdMmYyyy => "DD/MM/YYYY",
            DateFormat::MmDdYyyy => "MM/DD/YYYY",
            DateFormat::YyyyMmDd => "YYYY-MM-DD",
        };
        let number_format_str = match request.number_format {
            NumberFormat::CommaPeriod => "1,234.56",
            NumberFormat::PeriodComma => "1.234,56",
            NumberFormat::SpaceComma => "1 234,56",
        };
        self.repository
            .update_preferences_v2(
                user_id,
                theme_str,
                date_format_str,
                number_format_str,
                &request.language,
                request.compact_mode,
                &request.dashboard_layout,
                request.color_theme,
            )
            .await
    }

    // ── Sessions ─────────────────────────────────────────────────────────────

    pub async fn list_sessions(&self, user_id: &Uuid, current_session_id: Option<Uuid>) -> Result<Vec<SessionResponse>, AppError> {
        let sessions = self.repository.list_sessions_for_user(user_id).await?;
        let responses = sessions
            .into_iter()
            .map(|s| SessionResponse {
                id: s.id,
                created_at: s.created_at,
                is_current: current_session_id == Some(s.id),
            })
            .collect();
        Ok(responses)
    }

    pub async fn revoke_session(&self, session_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.delete_session_for_user(session_id, user_id).await
    }

    // ── Export ────────────────────────────────────────────────────────────────

    pub async fn export_transactions_csv(&self, user_id: &Uuid) -> Result<String, AppError> {
        let rows = self.repository.export_transactions_v2(user_id).await?;
        let mut csv = String::from("date,description,amount,currency,category,type,from_account,to_account,vendor\n");

        for row in &rows {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                row.date,
                escape_csv(&row.description),
                row.amount,
                escape_csv(&row.currency),
                escape_csv(&row.category),
                escape_csv(&row.tx_type),
                escape_csv(&row.from_account),
                escape_csv(&row.to_account),
                escape_csv(&row.vendor),
            ));
        }

        Ok(csv)
    }

    pub async fn export_data(&self, user_id: &Uuid) -> Result<serde_json::Value, AppError> {
        self.repository.export_all_data_v2(user_id).await
    }

    pub async fn import_data(&self, user_id: &Uuid, data: &serde_json::Value) -> Result<serde_json::Value, AppError> {
        let (accounts, categories, transactions) = self.repository.import_data_v2(user_id, data).await?;
        Ok(serde_json::json!({
            "imported": {
                "accounts": accounts,
                "categories": categories,
                "transactions": transactions,
            }
        }))
    }

    // ── Destructive ──────────────────────────────────────────────────────────

    pub async fn verify_password(&self, user_id: &Uuid, password: &str) -> Result<(), AppError> {
        let user = self.repository.get_user_by_id(user_id).await?.ok_or(AppError::UserNotFound)?;
        self.repository.verify_password(&user, password).await.map_err(|_| AppError::InvalidCredentials)
    }

    pub async fn reset_structure(&self, user_id: &Uuid, password: &str) -> Result<(), AppError> {
        self.verify_password(user_id, password).await?;
        self.repository.reset_structure_v2(user_id).await
    }

    pub async fn delete_account(&self, user_id: &Uuid, password: &str) -> Result<(), AppError> {
        self.verify_password(user_id, password).await?;
        // Must clean up data before deleting user because some FK constraints are RESTRICT
        self.repository.delete_all_user_data(user_id).await?;
        self.repository.delete_user(user_id).await
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
