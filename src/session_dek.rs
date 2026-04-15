//! Per-session DEK store and request guard.
//!
//! See `.kiro/specs/encryption-at-rest/design.md` §"DEK transport" for the
//! design. The v1 implementation is an in-process `Arc<RwLock<HashMap>>`
//! keyed by session_id; it is registered as Rocket managed state by
//! `build_rocket`. Phase 5 replaces the in-process store with a
//! Redis-backed one.
//!
//! Flow:
//!   1. User logs in → Rocket sets a session cookie. No DEK yet.
//!   2. Client POSTs the plaintext DEK to `/v2/auth/unlock`.
//!      The unlock handler calls `SessionDekStore::put` keyed by session_id.
//!   3. Subsequent authenticated requests that need encryption accept a
//!      `Dek` parameter via the `FromRequest` guard below, which reads the
//!      session cookie, looks up the DEK, and returns a cloned copy.
//!   4. Logout deletes both the session row and the store entry.
//!
//! The `Dek` type is zeroize-on-drop, so clones from the store are cleaned
//! up automatically when the handler returns.

use crate::auth::parse_session_cookie_value;
use crate::crypto::Dek;
use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-process session → DEK mapping.
#[derive(Clone)]
pub struct SessionDekStore {
    inner: Arc<RwLock<HashMap<Uuid, Dek>>>,
}

impl SessionDekStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a DEK for the given session. Overwrites any existing entry.
    /// The old Dek is zeroed on drop automatically.
    pub async fn put(&self, session_id: Uuid, dek: Dek) {
        let mut guard = self.inner.write().await;
        guard.insert(session_id, dek);
    }

    /// Return an owned clone of the DEK for the given session if present.
    /// The clone is a fresh `Dek` that the caller owns; it is zeroed on drop.
    pub async fn get_cloned(&self, session_id: &Uuid) -> Option<Dek> {
        let guard = self.inner.read().await;
        guard.get(session_id).map(|d| d.clone_for_request())
    }

    /// Remove the DEK entry for a session. Used on logout and session
    /// expiry. The removed Dek is dropped (and zeroed) synchronously.
    pub async fn remove(&self, session_id: &Uuid) {
        let mut guard = self.inner.write().await;
        guard.remove(session_id);
    }

    /// Remove every session entry for a given user. Used during password
    /// change to invalidate DEK caches on other devices, and during
    /// `delete_all_user_data`. We can't key by user_id directly because the
    /// map is session-keyed, so callers must pass the list of session ids
    /// they want invalidated.
    ///
    /// This helper is a placeholder until Phase 5 adds a secondary
    /// `session_id → user_id` index to the store.
    #[allow(dead_code)]
    pub async fn remove_many(&self, session_ids: &[Uuid]) {
        let mut guard = self.inner.write().await;
        for id in session_ids {
            guard.remove(id);
        }
    }
}

impl Default for SessionDekStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Request guard that yields the session DEK to route handlers.
///
/// Usage: add a `dek: Dek` parameter to any handler that needs to
/// encrypt or decrypt. The guard rejects the request with 401 if:
///   * there is no session cookie (user is not logged in), or
///   * the session cookie is present but the store has no DEK for it
///     (user is logged in but has not yet completed the unlock flow).
///
/// The guard does NOT validate session expiry; pair it with
/// `CurrentUser` on the same handler if you need both. Rocket runs
/// guards in parameter order, so session validity is checked by
/// `CurrentUser` before `Dek` runs.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Dek {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let store = match req.rocket().state::<SessionDekStore>() {
            Some(s) => s,
            None => return Outcome::Error((Status::InternalServerError, AppError::internal("SessionDekStore not registered"))),
        };

        let cookies = req.cookies();
        let Some(cookie) = cookies.get_private("user") else {
            return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
        };
        let Some((session_id, _user_id)) = parse_session_cookie_value(cookie.value()) else {
            return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
        };

        match store.get_cloned(&session_id).await {
            Some(dek) => Outcome::Success(dek),
            None => Outcome::Error((Status::Unauthorized, AppError::Unauthorized)),
        }
    }
}
