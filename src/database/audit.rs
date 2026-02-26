use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use serde_json::Value as JsonValue;
use uuid::Uuid;

impl PostgresRepository {
    /// Create a security audit log entry and log it to tracing
    pub async fn create_security_audit_log(
        &self,
        user_id: Option<&Uuid>,
        event_type: &str,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: Option<JsonValue>,
    ) -> Result<(), AppError> {
        // Log to tracing (stdout) for operational visibility (e.g. alerting
        // on login_failed spikes). PII — user_id, ip_address, user_agent —
        // is intentionally omitted here; it is persisted in the DB, which is
        // the authoritative audit record and access-controlled separately.
        if success {
            tracing::info!(
                category = "audit",
                event_type = event_type,
                success = success,
                "security audit event"
            );
        } else {
            tracing::warn!(
                category = "audit",
                event_type = event_type,
                success = success,
                "security audit event (failure)"
            );
        }

        sqlx::query(
            r#"
            INSERT INTO security_audit_log (user_id, event_type, success, ip_address, user_agent, metadata)
            VALUES ($1, $2, $3, $4::inet, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(event_type)
        .bind(success)
        .bind(ip_address)
        .bind(user_agent)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
