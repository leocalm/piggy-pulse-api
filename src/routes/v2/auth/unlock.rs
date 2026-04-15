use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::crypto::Dek;
use crate::error::app_error::AppError;
use crate::session_dek::SessionDekStore;

/// Request body for `POST /v2/auth/unlock`. Carries the plaintext DEK
/// that the client just unwrapped locally using the user's password.
///
/// The DEK is base64-encoded (32 raw bytes → 44 characters). Any other
/// length is a client error.
///
/// Intentionally does NOT derive `Debug` — we don't want the DEK bytes
/// (even as encoded string) appearing in panic messages or trace spans.
#[derive(Deserialize)]
pub struct UnlockRequest {
    pub dek: String,
}

/// `POST /v2/auth/unlock` — establishes the per-session DEK for this
/// cookie-authenticated session.
///
/// Client flow:
///   1. Log in via `POST /v2/auth/login`. Response body includes the
///      user's `wrapped_dek` and `dek_wrap_params` (or null on first login
///      for a legacy user).
///   2. Derive KEK locally via Argon2id(password, salt_from_params).
///   3. Unwrap `wrapped_dek` with KEK → 32-byte DEK.
///   4. Base64-encode the DEK and POST it to this endpoint.
///   5. Subsequent authenticated requests have the DEK available in the
///      session store via the `Dek` request guard.
///
/// The plaintext DEK only exists in:
///   * client memory between derive and upload
///   * the Rocket request body buffer briefly
///   * the in-process `SessionDekStore` for the session lifetime
///
/// Returns 204 on success, 400 on malformed input, 401 if the session
/// cookie is missing or invalid. Unlock is idempotent — calling it with
/// a different DEK simply overwrites the previous entry.
#[post("/unlock", data = "<payload>")]
pub async fn unlock(user: CurrentUser, store: &State<SessionDekStore>, payload: Json<UnlockRequest>) -> Result<Status, AppError> {
    let session_id = user.session_id.ok_or_else(|| {
        // Bearer-token callers cannot unlock. Only cookie-based sessions
        // are eligible; enforce at the boundary.
        AppError::BadRequest("unlock is only available for cookie-based sessions".to_string())
    })?;

    let raw = BASE64
        .decode(payload.dek.as_bytes())
        .map_err(|_| AppError::BadRequest("dek is not valid base64".to_string()))?;

    if raw.len() != 32 {
        return Err(AppError::BadRequest("dek must decode to exactly 32 bytes".to_string()));
    }

    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&raw);
    let dek = Dek::from_bytes(bytes);

    store.put(session_id, dek).await;

    // Drop the raw vec (was never sensitive beyond the copy we already
    // moved into the Dek), but be explicit anyway for memory hygiene.
    drop(raw);

    Ok(Status::NoContent)
}
