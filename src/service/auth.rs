// src/service/auth.rs

use crate::Config;
use crate::database::postgres_repository::PostgresRepository;
use uuid::Uuid;

/// What happened during a login attempt.
#[allow(dead_code)]
pub enum LoginOutcome {
    /// Credentials valid and session created successfully.
    Success { session_id: Uuid, user_id: Uuid },
    /// Credentials valid but 2FA code is required before a session is issued.
    TwoFactorRequired,
}

#[allow(dead_code)]
pub struct AuthService<'a> {
    pub repo: &'a PostgresRepository,
    pub config: &'a Config,
}
