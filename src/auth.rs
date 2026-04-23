use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use serde::Serialize;
use sha2::Digest;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum AuthMethod {
    Cookie,
    Bearer,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub session_id: Option<Uuid>,   // None for Bearer auth
    pub api_token_id: Option<Uuid>, // Set for Bearer auth — the DB row id
    pub auth_method: AuthMethod,
}

pub(crate) fn parse_session_cookie_value(value: &str) -> Option<(Uuid, Uuid)> {
    let (session_id_str, user_id_str) = value.split_once(':')?;
    let session_id = Uuid::parse_str(session_id_str).ok()?;
    let user_id = Uuid::parse_str(user_id_str).ok()?;
    Some((session_id, user_id))
}

impl CurrentUser {
    /// Returns the per-device auth principal id for this user: the session id
    /// for cookie auth, the api token row id for bearer auth. Used as the key
    /// for per-device server-side state (e.g. the session DEK store).
    ///
    /// Session ids and api token ids live in different tables but are both
    /// random UUIDs, so a single `Uuid` key space has no collision risk.
    pub fn principal_id(&self) -> Option<Uuid> {
        self.session_id.or(self.api_token_id)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let pool = match req.rocket().state::<PgPool>() {
            Some(pool) => pool,
            None => return Outcome::Error((Status::InternalServerError, AppError::Unauthorized)),
        };

        let repo = PostgresRepository { pool: pool.clone() };

        // Check Authorization header for Bearer token
        if let Some(auth_header) = req.headers().get_one("Authorization")
            && let Some(raw_token) = auth_header.strip_prefix("Bearer ")
        {
            let hash = hex::encode(sha2::Sha256::digest(raw_token.as_bytes()));

            match repo.find_by_access_hash(&hash).await {
                Ok(Some(token)) => {
                    if token.expires_at <= chrono::Utc::now() {
                        return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                    }

                    let _ = repo.touch(&token.id).await;

                    match repo.get_user_by_id(&token.user_id).await {
                        Ok(Some(user)) => {
                            let current_user = CurrentUser {
                                id: user.id,
                                username: user.email,
                                session_id: None,
                                api_token_id: Some(token.id),
                                auth_method: AuthMethod::Bearer,
                            };
                            req.local_cache(|| Some(current_user.clone()));
                            return Outcome::Success(current_user);
                        }
                        Ok(None) => {
                            return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                        }
                        Err(err) => {
                            return Outcome::Error((Status::InternalServerError, err));
                        }
                    }
                }
                Ok(None) => {
                    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                }
                Err(err) => {
                    return Outcome::Error((Status::InternalServerError, err));
                }
            }
        }

        // Fall through to cookie-based auth
        let cookies = req.cookies();
        if let Some(cookie) = cookies.get_private("user")
            && let Some((session_id, user_id)) = parse_session_cookie_value(cookie.value())
        {
            match repo.get_active_session_user(&session_id, &user_id).await {
                Ok(Some(user)) => {
                    let current_user = CurrentUser {
                        id: user.id,
                        username: user.email,
                        session_id: Some(session_id),
                        api_token_id: None,
                        auth_method: AuthMethod::Cookie,
                    };
                    req.local_cache(|| Some(current_user.clone()));
                    return Outcome::Success(current_user);
                }
                Ok(None) => {
                    // `Ok(None)` covers two cases that are indistinguishable
                    // without an extra DB round-trip: (a) the session row exists
                    // but has expired, or (b) the session was never created /
                    // already deleted. We record "expired_or_not_found" as the
                    // reason so forensic readers are not misled.
                    let _ = repo.delete_session_if_expired(&session_id).await;

                    let ip = req.client_ip().map(|ip| ip.to_string());
                    let ua = req.headers().get_one("User-Agent").map(|s| s.to_string());
                    if let Err(e) = repo
                        .create_security_audit_log(
                            Some(&user_id),
                            crate::models::audit::audit_events::SESSION_EXPIRED,
                            false,
                            ip,
                            ua,
                            Some(serde_json::json!({
                                "session_id": session_id.to_string(),
                                "reason": "expired_or_not_found"
                            })),
                        )
                        .await
                    {
                        tracing::warn!(error = %e, "Failed to persist session_expired audit event");
                    }

                    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                }
                Err(err) => return Outcome::Error((Status::InternalServerError, err)),
            }
        }

        Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials))
    }
}

/// Like `CurrentUser` but accepts bearer tokens that are expired (access)
/// as long as they are still within the `refresh_expires_at` window.
/// Used exclusively by the token refresh endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct RefreshableUser {
    pub id: Uuid,
    pub username: String,
    pub session_id: Option<Uuid>,
    pub api_token_id: Option<Uuid>,
    pub auth_method: AuthMethod,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RefreshableUser {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let pool = match req.rocket().state::<PgPool>() {
            Some(pool) => pool,
            None => return Outcome::Error((Status::InternalServerError, AppError::Unauthorized)),
        };

        let repo = PostgresRepository { pool: pool.clone() };

        // Check Authorization header for Bearer token
        if let Some(auth_header) = req.headers().get_one("Authorization")
            && let Some(raw_token) = auth_header.strip_prefix("Bearer ")
        {
            let hash = hex::encode(sha2::Sha256::digest(raw_token.as_bytes()));

            match repo.find_by_access_hash(&hash).await {
                Ok(Some(token)) => {
                    // Allow expired access tokens as long as refresh window is still valid
                    if token.refresh_expires_at <= chrono::Utc::now() {
                        return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                    }

                    if token.revoked_at.is_some() {
                        return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                    }

                    match repo.get_user_by_id(&token.user_id).await {
                        Ok(Some(user)) => {
                            return Outcome::Success(RefreshableUser {
                                id: user.id,
                                username: user.email,
                                session_id: None,
                                api_token_id: Some(token.id),
                                auth_method: AuthMethod::Bearer,
                            });
                        }
                        Ok(None) => {
                            return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                        }
                        Err(err) => {
                            return Outcome::Error((Status::InternalServerError, err));
                        }
                    }
                }
                Ok(None) => {
                    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                }
                Err(err) => {
                    return Outcome::Error((Status::InternalServerError, err));
                }
            }
        }

        // Fall through to cookie-based auth (same as CurrentUser)
        let cookies = req.cookies();
        if let Some(cookie) = cookies.get_private("user")
            && let Some((session_id, user_id)) = parse_session_cookie_value(cookie.value())
        {
            match repo.get_active_session_user(&session_id, &user_id).await {
                Ok(Some(user)) => {
                    return Outcome::Success(RefreshableUser {
                        id: user.id,
                        username: user.email,
                        session_id: Some(session_id),
                        api_token_id: None,
                        auth_method: AuthMethod::Cookie,
                    });
                }
                Ok(None) | Err(_) => {
                    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                }
            }
        }

        Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_session_cookie_value;
    use uuid::Uuid;

    #[test]
    fn parse_session_cookie_value_valid() {
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let value = format!("{}:{}", session_id, user_id);
        let parsed = parse_session_cookie_value(&value);
        assert!(matches!(parsed, Some((parsed_session_id, parsed_user_id)) if parsed_session_id == session_id && parsed_user_id == user_id));
    }

    #[test]
    fn parse_session_cookie_value_invalid_uuid() {
        let parsed = parse_session_cookie_value("not-a-uuid:user@example.com");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_session_cookie_value_missing_delimiter() {
        let parsed = parse_session_cookie_value("missing-delimiter");
        assert!(parsed.is_none());
    }

    #[test]
    fn bearer_prefix_stripped_correctly() {
        let header = "Bearer pp_at_abc123";
        let token = header.strip_prefix("Bearer ").unwrap();
        assert_eq!(token, "pp_at_abc123");
    }

    #[test]
    fn bearer_without_space_not_matched() {
        let header = "Bearerpp_at_abc123";
        assert!(header.strip_prefix("Bearer ").is_none());
    }
}
