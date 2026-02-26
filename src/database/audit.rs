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
        // Log to tracing (stdout) as well for operational visibility
        let uid_str = user_id.map(|u| u.to_string());
        if success {
            tracing::info!(
                category = "audit",
                event_type = event_type,
                success = success,
                user_id = uid_str.as_deref().unwrap_or("-"),
                ip = ip_address.as_deref().unwrap_or("-"),
                user_agent = user_agent.as_deref().unwrap_or("-"),
                "security audit event"
            );
        } else {
            tracing::warn!(
                category = "audit",
                event_type = event_type,
                success = success,
                user_id = uid_str.as_deref().unwrap_or("-"),
                ip = ip_address.as_deref().unwrap_or("-"),
                user_agent = user_agent.as_deref().unwrap_or("-"),
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
