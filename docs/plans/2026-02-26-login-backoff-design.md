# Login Backoff Mechanism Design

## Overview
Implement a progressive backoff system for failed login attempts to protect against brute-force attacks while maintaining good user experience. The system uses a hybrid approach that tracks both user accounts and IP addresses, implementing progressive delays that eventually lead to temporary account lockout.

## Requirements Summary
- **Strategy**: Combination of progressive delays leading to temporary lockout
- **Thresholds**: 3 free attempts, then 5s, 30s, 60s delays, lockout after 7 attempts for 1 hour
- **Tracking**: Hybrid - by account when known, by IP for non-existent accounts
- **Enforcement**: Database-enforced with stored timestamps
- **Notifications**: Both user and admin notifications with configurable thresholds
- **Recovery**: Automatic expiry after timeout + optional email unlock link

## Architecture Decision
**Selected Approach**: Unified Rate Limit Table

We'll create a single `login_rate_limits` table that handles both account-based and IP-based tracking. This provides:
- Single source of truth for all rate limiting
- Clean integration with existing audit logging
- Flexibility to extend with new tracking dimensions
- Simpler queries and maintenance

## Database Schema

### New Table: `login_rate_limits`
```sql
CREATE TABLE login_rate_limits (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identifier_type   VARCHAR(10) NOT NULL, -- 'user_id' or 'ip_address'
    identifier_value  VARCHAR(255) NOT NULL, -- UUID for user or IP string
    failed_attempts   INTEGER NOT NULL DEFAULT 0,
    last_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_until      TIMESTAMPTZ NULL,      -- NULL = not locked
    next_attempt_at   TIMESTAMPTZ NULL,      -- Enforced delay timestamp
    unlock_token      VARCHAR(255) NULL,     -- For email-based unlock
    unlock_token_expires_at TIMESTAMPTZ NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes for performance
CREATE UNIQUE INDEX idx_login_rate_limits_identifier
    ON login_rate_limits(identifier_type, identifier_value);
CREATE INDEX idx_login_rate_limits_locked
    ON login_rate_limits(locked_until) WHERE locked_until IS NOT NULL;
CREATE INDEX idx_login_rate_limits_next_attempt
    ON login_rate_limits(next_attempt_at) WHERE next_attempt_at IS NOT NULL;
```

## Backend Implementation

### Core Methods in `PostgresRepository`

1. **`check_login_rate_limit(user_id: Option<Uuid>, ip_address: &str) -> RateLimitStatus`**
   - Checks both user and IP rate limits
   - Returns: `Allowed`, `Delayed(until)`, or `Locked(until)`

2. **`record_failed_login_attempt(user_id: Option<Uuid>, ip_address: &str)`**
   - Increments failure counter
   - Calculates and applies delays/lockouts
   - Delay progression: attempts 1-3 free, #4 = 5s, #5 = 30s, #6 = 60s, #7+ = locked

3. **`reset_login_rate_limit(user_id: &Uuid, ip_address: &str)`**
   - Called on successful login
   - Clears failed attempts for both user and IP

4. **`send_unlock_email(user_id: &Uuid)`**
   - Generates secure unlock token
   - Sends email with unlock link
   - Token expires after 1 hour

### Login Endpoint Flow

```rust
// Pseudo-code for login endpoint
async fn post_user_login() {
    // 1. Check rate limits BEFORE password verification
    let rate_limit_status = repo.check_login_rate_limit(user_id, ip_address).await?;

    match rate_limit_status {
        RateLimitStatus::Allowed => {
            // 2. Proceed with normal login flow
            if verify_password() {
                // 3. Reset rate limits on success
                repo.reset_login_rate_limit(user_id, ip_address).await?;
            } else {
                // 4. Record failed attempt
                repo.record_failed_login_attempt(user_id, ip_address).await?;
            }
        },
        RateLimitStatus::Delayed { until } => {
            // Return 429 with retry-after header
        },
        RateLimitStatus::Locked { until } => {
            // Return 423 Locked, send unlock email if first time
        }
    }
}
```

## API Responses

### Error Response Types

1. **Rate Limited (429 Too Many Requests)**
   ```json
   {
     "error": "too_many_attempts",
     "message": "Too many failed attempts. Please wait before trying again.",
     "retry_after_seconds": 30
   }
   ```

2. **Account Locked (423 Locked)**
   ```json
   {
     "error": "account_locked",
     "message": "Account temporarily locked. Check email for unlock instructions.",
     "locked_until": "2026-02-26T15:00:00Z"
   }
   ```

## Configuration

### Environment Variables
```env
# Rate Limiting Configuration
RATE_LIMIT_FREE_ATTEMPTS=3
RATE_LIMIT_DELAYS=5,30,60
RATE_LIMIT_LOCKOUT_ATTEMPTS=7
RATE_LIMIT_LOCKOUT_MINUTES=60
RATE_LIMIT_ENABLE_EMAIL_UNLOCK=true
RATE_LIMIT_NOTIFY_USER_ON_LOCK=true
RATE_LIMIT_NOTIFY_ADMIN_ON_LOCK=true
RATE_LIMIT_ADMIN_EMAIL=admin@piggy-pulse.com
RATE_LIMIT_HIGH_FAILURE_THRESHOLD=20
```

## Notification System

### User Notifications
- **Trigger**: Account locked after 7 failed attempts
- **Content**: Email with temporary unlock link and security advice
- **Unlock link format**: `/unlock?token={token}&user={user_id}`

### Admin Notifications
- **Triggers**:
  - Any account locked
  - High failure rate (20+ failures/hour) from single IP
- **Content**: Details about the security event and affected account/IP

### Audit Log Events
New event types for `security_audit_log`:
- `LOGIN_RATE_LIMITED` - Delay enforced due to failed attempts
- `ACCOUNT_LOCKED` - Account locked after exceeding threshold
- `ACCOUNT_UNLOCKED` - Unlock link successfully used
- `HIGH_FAILURE_RATE` - Abnormal failure rate detected

## Frontend Integration

### Login Form Enhancements
1. Display countdown timer during delay period
2. Disable form submission while rate limited
3. Show clear messaging about lock status
4. Provide "Request unlock email" button when locked

### Error Handling
```typescript
// In auth API client
async function login(credentials) {
  try {
    const response = await api.post('/users/login', credentials);
    return response.data;
  } catch (error) {
    if (error.response?.status === 429) {
      // Handle rate limit - show countdown
      const retryAfter = error.response.data.retry_after_seconds;
      showDelayMessage(retryAfter);
    } else if (error.response?.status === 423) {
      // Handle account lock
      showLockMessage(error.response.data.locked_until);
    }
    throw error;
  }
}
```

## Security Considerations

1. **Timing Attack Prevention**: Continue using constant-time password verification
2. **IP Spoofing**: Use reliable IP detection (consider X-Forwarded-For with trusted proxies)
3. **Distributed Attacks**: IP-based limiting helps against attacks from multiple IPs
4. **Token Security**: Unlock tokens are single-use and expire after 1 hour
5. **Audit Trail**: All rate limit events logged for security monitoring

## Testing Strategy

1. **Unit Tests**:
   - Rate limit calculation logic
   - Delay progression
   - Token generation and validation

2. **Integration Tests**:
   - Full login flow with rate limiting
   - Email notification sending
   - Unlock token flow

3. **E2E Tests**:
   - User experience during rate limiting
   - Countdown timer display
   - Unlock email flow

## Migration Plan

1. Create new database table and indexes
2. Deploy backend code with feature flag
3. Test in staging environment
4. Enable gradually with monitoring
5. Update frontend to handle new error responses

## Success Metrics

- Reduction in brute force attempts (measured via audit logs)
- No increase in legitimate user lockouts
- Successful unlock rate > 90%
- Admin response time to security alerts < 15 minutes

## Future Enhancements

- CAPTCHA integration after N failed attempts
- Machine learning for anomaly detection
- Geographic-based rate limiting
- Configurable per-user security levels