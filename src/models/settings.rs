use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

// ── Existing general settings ─────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct Settings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_currency_id: Option<Uuid>,
    pub budget_stability_tolerance_basis_points: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct SettingsResponse {
    pub id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_currency_id: Option<Uuid>,
    pub budget_stability_tolerance_basis_points: i32,
    pub updated_at: DateTime<Utc>,
}

impl From<&Settings> for SettingsResponse {
    fn from(value: &Settings) -> Self {
        Self {
            id: value.id,
            theme: value.theme.clone(),
            language: value.language.clone(),
            default_currency_id: value.default_currency_id,
            budget_stability_tolerance_basis_points: value.budget_stability_tolerance_basis_points,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct SettingsRequest {
    #[validate(length(min = 1))]
    #[schemars(regex(pattern = r"^(light|dark|auto)$"))]
    pub theme: String,

    #[validate(length(equal = 2))]
    #[schemars(regex(pattern = r"^(en|es|pt|fr|de)$"))]
    pub language: String,

    pub default_currency_id: Option<Uuid>,

    #[validate(range(min = 0, max = 10000))]
    pub budget_stability_tolerance_basis_points: Option<i32>,
}

// ── Profile ───────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct ProfileData {
    pub name: String,
    pub email: String,
    pub timezone: String,
    pub default_currency_id: Option<Uuid>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct ProfileResponse {
    pub name: String,
    /// Email is masked; use the email-change flow to update it.
    pub email: String,
    pub timezone: String,
    pub default_currency_id: Option<Uuid>,
}

impl From<&ProfileData> for ProfileResponse {
    fn from(d: &ProfileData) -> Self {
        Self {
            name: d.name.clone(),
            email: mask_email(&d.email),
            timezone: d.timezone.clone(),
            default_currency_id: d.default_currency_id,
        }
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct ProfileRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(custom(function = "validate_timezone"))]
    pub timezone: String,
    pub default_currency_id: Option<Uuid>,
}

fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let masked = if local.len() > 1 { format!("{}***", &local[..1]) } else { "***".to_string() };
            format!("{}@{}", masked, domain)
        }
        None => "***".to_string(),
    }
}

// ── Preferences ───────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct UserPreferences {
    pub theme: String,
    pub date_format: String,
    pub number_format: String,
    pub compact_mode: bool,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct PreferencesResponse {
    pub theme: String,
    pub date_format: String,
    pub number_format: String,
    pub compact_mode: bool,
}

impl From<&UserPreferences> for PreferencesResponse {
    fn from(p: &UserPreferences) -> Self {
        Self {
            theme: p.theme.clone(),
            date_format: p.date_format.clone(),
            number_format: p.number_format.clone(),
            compact_mode: p.compact_mode,
        }
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct PreferencesRequest {
    #[validate(custom(function = "validate_theme"))]
    pub theme: String,
    #[validate(custom(function = "validate_date_format"))]
    pub date_format: String,
    #[validate(custom(function = "validate_number_format"))]
    pub number_format: String,
    pub compact_mode: bool,
}

// ── Password change ───────────────────────────────────────────────────────────

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct PasswordChangeRequest {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "crate::models::user::validate_password_strength"))]
    pub new_password: String,
}

// ── Sessions ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Debug, JsonSchema)]
pub struct SessionInfoResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

// ── Period model ──────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
pub struct PeriodSchedule {
    pub start_day: i32,
    pub duration_value: i32,
    pub duration_unit: String,
    pub saturday_adjustment: String,
    pub sunday_adjustment: String,
    pub name_pattern: String,
    pub generate_ahead: i32,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct ScheduleConfigResponse {
    pub start_day: i32,
    pub duration_value: i32,
    pub duration_unit: String,
    pub saturday_adjustment: String,
    pub sunday_adjustment: String,
    pub name_pattern: String,
    pub generate_ahead: i32,
}

impl From<&PeriodSchedule> for ScheduleConfigResponse {
    fn from(s: &PeriodSchedule) -> Self {
        Self {
            start_day: s.start_day,
            duration_value: s.duration_value,
            duration_unit: s.duration_unit.clone(),
            saturday_adjustment: s.saturday_adjustment.clone(),
            sunday_adjustment: s.sunday_adjustment.clone(),
            name_pattern: s.name_pattern.clone(),
            generate_ahead: s.generate_ahead,
        }
    }
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct PeriodModelResponse {
    pub mode: String,
    pub schedule: Option<ScheduleConfigResponse>,
}

fn validate_timezone(tz: &str) -> Result<(), ValidationError> {
    if tz.parse::<chrono_tz::Tz>().is_ok() {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_timezone"))
    }
}

fn validate_theme(theme: &str) -> Result<(), ValidationError> {
    if matches!(theme, "light" | "dark" | "auto") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_theme"))
    }
}

fn validate_date_format(fmt: &str) -> Result<(), ValidationError> {
    if matches!(fmt, "DD/MM/YYYY" | "MM/DD/YYYY" | "YYYY-MM-DD" | "DD-MM-YYYY" | "DD.MM.YYYY") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_date_format"))
    }
}

fn validate_number_format(fmt: &str) -> Result<(), ValidationError> {
    if matches!(fmt, "1,234.56" | "1.234,56" | "1 234,56" | "1 234.56") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_number_format"))
    }
}

fn validate_period_mode(mode: &str) -> Result<(), ValidationError> {
    if matches!(mode, "automatic" | "manual") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_period_mode"))
    }
}

fn validate_duration_unit(unit: &str) -> Result<(), ValidationError> {
    if matches!(unit, "days" | "weeks" | "months") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_duration_unit"))
    }
}

fn validate_weekend_adj(adj: &str) -> Result<(), ValidationError> {
    if matches!(adj, "keep" | "friday" | "monday") {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_weekend_adjustment"))
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct ScheduleConfig {
    #[validate(range(min = 1, max = 31))]
    pub start_day: i32,
    #[validate(range(min = 1))]
    pub duration_value: i32,
    #[validate(custom(function = "validate_duration_unit"))]
    pub duration_unit: String,
    #[validate(custom(function = "validate_weekend_adj"))]
    pub saturday_adjustment: String,
    #[validate(custom(function = "validate_weekend_adj"))]
    pub sunday_adjustment: String,
    #[validate(length(min = 1))]
    pub name_pattern: String,
    #[validate(range(min = 0))]
    pub generate_ahead: i32,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct PeriodModelRequest {
    #[validate(custom(function = "validate_period_mode"))]
    pub mode: String,
    #[validate(nested)]
    pub schedule: Option<ScheduleConfig>,
}

// ── Danger zone ───────────────────────────────────────────────────────────────

fn validate_delete_confirmation(s: &str) -> Result<(), ValidationError> {
    if s == "DELETE" {
        Ok(())
    } else {
        Err(ValidationError::new("confirmation must equal 'DELETE'"))
    }
}

fn validate_reset_confirmation(s: &str) -> Result<(), ValidationError> {
    if s == "RESET" {
        Ok(())
    } else {
        Err(ValidationError::new("confirmation must equal 'RESET'"))
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct DeleteAccountRequest {
    /// Must equal "DELETE" to confirm destructive action.
    #[validate(custom(function = "validate_delete_confirmation"))]
    pub confirmation: String,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct ResetStructureRequest {
    /// Must equal "RESET" to confirm destructive action.
    #[validate(custom(function = "validate_reset_confirmation"))]
    pub confirmation: String,
}
