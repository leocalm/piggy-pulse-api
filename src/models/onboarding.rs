use rocket::serde::Serialize;
use schemars::JsonSchema;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStatus {
    NotStarted,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    Period,
    Accounts,
    Categories,
    Summary,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct OnboardingStatusResponse {
    pub status: OnboardingStatus,
    pub current_step: Option<OnboardingStep>,
}
