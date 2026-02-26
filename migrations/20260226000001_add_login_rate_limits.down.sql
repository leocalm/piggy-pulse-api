-- Drop indexes
DROP INDEX IF EXISTS idx_login_rate_limits_unlock_token;
DROP INDEX IF EXISTS idx_login_rate_limits_next_attempt;
DROP INDEX IF EXISTS idx_login_rate_limits_locked;
DROP INDEX IF EXISTS idx_login_rate_limits_identifier;

-- Drop table
DROP TABLE IF EXISTS login_rate_limits;