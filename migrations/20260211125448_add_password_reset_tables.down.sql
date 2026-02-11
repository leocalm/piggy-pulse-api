-- Drop cleanup function
DROP FUNCTION IF EXISTS cleanup_expired_password_resets();

-- Drop indexes
DROP INDEX IF EXISTS idx_security_audit_log_success;
DROP INDEX IF EXISTS idx_security_audit_log_created_at;
DROP INDEX IF EXISTS idx_security_audit_log_event_type;
DROP INDEX IF EXISTS idx_security_audit_log_user_id;

DROP INDEX IF EXISTS idx_password_resets_expires_at;
DROP INDEX IF EXISTS idx_password_resets_token_hash;
DROP INDEX IF EXISTS idx_password_resets_user_id;

-- Drop tables
DROP TABLE IF EXISTS security_audit_log;
DROP TABLE IF EXISTS password_resets;
