// Single-endpoint domains (health, unlock) use flat files.
// All other domains use a subdirectory with mod.rs + one file per OpenAPI path.

pub mod accounts;
pub mod auth;
pub mod categories;
pub mod currencies;
pub mod dashboard;
pub mod health;
pub mod onboarding;
pub mod periods;
pub mod settings;
pub mod subscriptions;
pub mod targets;
pub mod transactions;
pub mod unlock;
pub mod vendors;
