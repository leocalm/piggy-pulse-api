//! Per-session DEK store and request guard.
//!
//! See `.kiro/specs/encryption-at-rest/design.md` §"DEK transport" for the
//! design. The v1 implementation is an in-process `Arc<RwLock<HashMap>>`
//! keyed by the per-device auth principal id — session_id for cookie auth
//! (web), api_token_id for bearer auth (mobile). It is registered as Rocket
//! managed state by `build_rocket`. Phase 5 replaces the in-process store
//! with a Redis-backed one.
//!
//! Flow:
//!   1. User logs in → cookie session or bearer token is issued. No DEK yet.
//!   2. Client POSTs the plaintext DEK to `/v2/auth/unlock`.
//!      The unlock handler calls `SessionDekStore::put` keyed by the
//!      principal id (`CurrentUser::principal_id`).
//!   3. Subsequent authenticated requests that need encryption accept a
//!      `Dek` parameter via the `FromRequest` guard below, which resolves
//!      `CurrentUser` and looks up the DEK by that same principal id.
//!   4. Logout / session revoke deletes the store entry alongside the
//!      session/token row.
//!
//! The `Dek` type is zeroize-on-drop, so clones from the store are cleaned
//! up automatically when the handler returns.

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-process principal → DEK mapping. The principal id is either a
/// session_id (cookie auth) or an api_token_id (bearer auth); both are
/// random UUIDs drawn from non-overlapping tables.
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

    /// Store a DEK for the given principal. Overwrites any existing entry.
    /// The old Dek is zeroed on drop automatically.
    pub async fn put(&self, principal_id: Uuid, dek: Dek) {
        let mut guard = self.inner.write().await;
        guard.insert(principal_id, dek);
    }

    /// Return an owned clone of the DEK for the given principal if present.
    /// The clone is a fresh `Dek` that the caller owns; it is zeroed on drop.
    pub async fn get_cloned(&self, principal_id: &Uuid) -> Option<Dek> {
        let guard = self.inner.read().await;
        guard.get(principal_id).map(|d| d.clone_for_request())
    }

    /// Remove the DEK entry for a principal. Used on logout, session
    /// revoke, and session expiry. The removed Dek is dropped (and zeroed)
    /// synchronously.
    pub async fn remove(&self, principal_id: &Uuid) {
        let mut guard = self.inner.write().await;
        guard.remove(principal_id);
    }

    /// Remove every entry for a given user. Used during password change to
    /// invalidate DEK caches on other devices, and during
    /// `delete_all_user_data`. We can't key by user_id directly because the
    /// map is principal-keyed, so callers must pass the list of principal
    /// ids (session ids and/or api token ids) they want invalidated.
    ///
    /// This helper is a placeholder until Phase 5 adds a secondary
    /// `principal_id → user_id` index to the store.
    #[allow(dead_code)]
    pub async fn remove_many(&self, principal_ids: &[Uuid]) {
        let mut guard = self.inner.write().await;
        for id in principal_ids {
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
/// encrypt or decrypt. The guard resolves `CurrentUser` to identify the
/// authenticated principal (cookie session or bearer token) and looks up
/// the DEK by that principal's id. Rejects with 401 if:
///   * the caller is not authenticated, or
///   * the caller is authenticated but has not yet completed `/v2/auth/unlock`.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Dek {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let store = match req.rocket().state::<SessionDekStore>() {
            Some(s) => s,
            None => return Outcome::Error((Status::InternalServerError, AppError::internal("SessionDekStore not registered"))),
        };

        let user = match req.guard::<CurrentUser>().await {
            Outcome::Success(u) => u,
            Outcome::Error(e) => return Outcome::Error(e),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };

        let Some(principal_id) = user.principal_id() else {
            return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
        };

        match store.get_cloned(&principal_id).await {
            Some(dek) => Outcome::Success(dek),
            None => Outcome::Error((Status::Unauthorized, AppError::Unauthorized)),
        }
    }
}
