ALTER TABLE user_session
    DROP COLUMN IF EXISTS user_agent,
    DROP COLUMN IF EXISTS ip_address;
