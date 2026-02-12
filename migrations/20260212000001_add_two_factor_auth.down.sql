-- Rollback two-factor authentication tables
-- Drop in reverse order to respect foreign key constraints

DROP INDEX IF EXISTS idx_emergency_tokens_expires;
DROP INDEX IF EXISTS idx_emergency_tokens_user_id;
DROP INDEX IF EXISTS idx_rate_limit_locked;
DROP INDEX IF EXISTS idx_rate_limit_user_unique;
DROP INDEX IF EXISTS idx_rate_limit_user_id;
DROP INDEX IF EXISTS idx_backup_codes_user_id_used;
DROP INDEX IF EXISTS idx_two_factor_auth_user_id;

DROP TABLE IF EXISTS two_factor_emergency_tokens;
DROP TABLE IF EXISTS two_factor_rate_limits;
DROP TABLE IF EXISTS two_factor_backup_codes;
DROP TABLE IF EXISTS two_factor_auth;
