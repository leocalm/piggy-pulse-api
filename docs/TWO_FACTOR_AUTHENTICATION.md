# Two-Factor Authentication (2FA) Documentation

## Table of Contents

1. [Overview](#overview)
2. [User Guide](#user-guide)
3. [Configuration](#configuration)
4. [Security Considerations](#security-considerations)
5. [Emergency Recovery](#emergency-recovery)
6. [API Reference](#api-reference)
7. [Testing](#testing)

---

## Overview

PiggyPulse implements Time-based One-Time Password (TOTP) two-factor authentication to provide an additional layer of security for user accounts. This implementation follows industry best practices and is compatible with popular authenticator apps like Google Authenticator, Authy, 1Password, and others.

### Features

- **TOTP-based 2FA**: Industry-standard time-based one-time passwords
- **QR Code Setup**: Easy setup via QR code scanning
- **Backup Codes**: 10 single-use backup codes for emergency access
- **Emergency Disable**: Email-based emergency 2FA disable for lost devices
- **Rate Limiting**: Protection against brute-force attacks
- **Encrypted Storage**: TOTP secrets stored with AES-256-GCM encryption

---

## User Guide

### Enabling 2FA

1. **Navigate to Settings**
   - Log in to your account
   - Go to Settings → Security → Two-Factor Authentication

2. **Setup Process**
   - Click "Enable 2FA"
   - Scan the QR code with your authenticator app
   - Save your backup codes in a secure location
   - Enter the 6-digit code from your app to verify
   - Click "Verify & Enable"

3. **Supported Authenticator Apps**
   - Google Authenticator (iOS, Android)
   - Authy (iOS, Android, Desktop)
   - 1Password (iOS, Android, Browser Extension)
   - Microsoft Authenticator
   - Any TOTP-compatible authenticator

### Using 2FA

After enabling 2FA, you'll be prompted for a 6-digit code when logging in:

1. Enter your email and password
2. When prompted, open your authenticator app
3. Enter the 6-digit code shown for PiggyPulse
4. Click "Verify"

**Alternative: Backup Codes**
- If you don't have access to your authenticator app, click "Use backup code instead"
- Enter one of your saved backup codes
- Each backup code can only be used once

### Managing Backup Codes

**Viewing Remaining Codes**
- Go to Settings → Security → Two-Factor Authentication
- Your remaining backup codes count is displayed

**Regenerating Backup Codes**
- Click "Regenerate Backup Codes"
- Enter your current 2FA code
- Save the new set of 10 backup codes
- Previous backup codes will be invalidated

**Important Notes**
- Each backup code can only be used once
- When you run low (< 3 remaining), regenerate them
- Store backup codes securely (password manager, encrypted file, or printed in a safe place)

### Disabling 2FA

**Standard Disable**
1. Go to Settings → Security → Two-Factor Authentication
2. Click "Disable 2FA"
3. Enter your password
4. Enter your current 2FA code
5. Confirm the action

**Emergency Disable** (if you lost your authenticator device)
1. On the login page, when prompted for 2FA code, click "Lost access to your authenticator?"
2. Enter your email address
3. Check your email for the emergency disable link
4. Click the link to confirm disabling 2FA
5. You can now log in with just your password

---

## Configuration

### Backend Configuration

Add the following to your `Budget.toml`:

```toml
[two_factor]
# Generate a secure key with: openssl rand -hex 32
# WARNING: Change this key in production!
encryption_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
issuer_name = "PiggyPulse"
frontend_emergency_disable_url = "http://localhost:5173/auth/emergency-2fa-disable"
```

### Environment Variables

You can override configuration using environment variables:

```bash
# Encryption key for TOTP secrets
export BUDGET_TWO_FACTOR__ENCRYPTION_KEY="your-64-char-hex-key-here"

# Issuer name shown in authenticator apps
export BUDGET_TWO_FACTOR__ISSUER_NAME="PiggyPulse"

# Frontend URL for emergency disable confirmation
export BUDGET_TWO_FACTOR__FRONTEND_EMERGENCY_DISABLE_URL="https://yourdomain.com/auth/emergency-2fa-disable"
```

### Generating a Secure Encryption Key

**CRITICAL**: Never use the default encryption key in production!

```bash
# Generate a new encryption key
openssl rand -hex 32

# Example output (DO NOT USE THIS):
# a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v2w3x4y5z6a7b8c9d0e1f2
```

### Email Configuration

2FA emergency disable requires email to be configured. Add to `Budget.toml`:

```toml
[email]
smtp_host = "smtp.example.com"
smtp_port = 465
smtp_username = "noreply"
smtp_password = "your-smtp-password"
from_address = "noreply@yourapp.com"
from_name = "YourApp"
enabled = true
```

---

## Security Considerations

### Encryption

- **Algorithm**: AES-256-GCM
- **Key Derivation**: Direct 32-byte key (must be securely generated)
- **Nonce**: 12-byte random nonce per encryption operation
- **Storage**: Encrypted secrets and nonces stored in PostgreSQL

### Rate Limiting

Protection against brute-force attacks:
- **Failed Attempts Limit**: 5 failed attempts
- **Lockout Period**: 15 minutes
- **Scope**: Per user account
- **Reset**: Automatic after successful authentication

### Token Expiry

- **TOTP Time Step**: 30 seconds (industry standard)
- **Time Window**: ±1 step (allows for clock drift)
- **Emergency Disable Token**: 1 hour expiry
- **Backup Codes**: No expiration (until used or regenerated)

### Session Management

When 2FA is enabled or disabled:
- **No Session Invalidation**: Existing sessions remain valid
- **Future Logins**: Require 2FA code for new logins
- **Recommendation**: Consider logging out all devices after enabling 2FA

### Best Practices

1. **Encryption Key**
   - Generate using cryptographically secure random generator
   - Store securely (environment variable, secrets manager)
   - Never commit to version control
   - Rotate periodically (requires re-setup for all users)

2. **Backup Codes**
   - Provide exactly 10 codes (industry standard)
   - Each code is single-use
   - Stored as bcrypt hashes (same as passwords)
   - Regeneration invalidates old codes

3. **Emergency Disable**
   - Requires email verification
   - 1-hour token expiry
   - Tokens are single-use
   - Consider security audit log review

4. **User Education**
   - Encourage backup code storage
   - Explain emergency disable process
   - Recommend using password manager for backup codes

---

## Emergency Recovery

### User Lost Authenticator Device

**Scenario**: User lost their phone/authenticator app and doesn't have backup codes.

**Solution**: Emergency Disable Flow

1. User goes to login page
2. Enters email and password
3. When prompted for 2FA, clicks "Lost access to your authenticator?"
4. Enters email address
5. Receives email with emergency disable link
6. Clicks link to disable 2FA
7. Can now log in with password only
8. Should immediately re-enable 2FA with new device

### Admin Recovery (Database Access)

**WARNING**: Only perform if absolutely necessary and with proper authorization.

```sql
-- Disable 2FA for a specific user
UPDATE two_factor_auth
SET is_enabled = false
WHERE user_id = 'USER_UUID_HERE';

-- View 2FA status for a user
SELECT
    u.email,
    tfa.is_enabled,
    tfa.created_at,
    tfa.verified_at
FROM users u
LEFT JOIN two_factor_auth tfa ON u.id = tfa.user_id
WHERE u.email = 'user@example.com';
```

### Encryption Key Loss

**Scenario**: The encryption key is lost or compromised.

**Impact**: All existing TOTP secrets become unreadable.

**Recovery**:
1. Generate a new encryption key
2. All users must re-setup 2FA
3. Consider sending notification email to all users with 2FA enabled
4. Update configuration with new key
5. Restart application

**Prevention**:
- Store encryption key in secrets manager (AWS Secrets Manager, HashiCorp Vault, etc.)
- Maintain secure backup of encryption key
- Document key rotation procedure

---

## API Reference

### Setup 2FA

**POST** `/api/v1/two-factor/setup`

Generates a new TOTP secret and backup codes for the authenticated user.

**Authentication**: Required

**Response**:
```json
{
  "secret": "JBSWY3DPEHPK3PXP",
  "qr_code": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0i...",
  "backup_codes": [
    "ABC-DEF-GHI",
    "JKL-MNO-PQR",
    ...
  ]
}
```

### Verify 2FA

**POST** `/api/v1/two-factor/verify`

Verifies the TOTP code and enables 2FA for the user.

**Authentication**: Required

**Request**:
```json
{
  "code": "123456"
}
```

**Response**: `200 OK` or `400 Bad Request`

### Disable 2FA

**DELETE** `/api/v1/two-factor/disable`

Disables 2FA for the user (requires password and current code).

**Authentication**: Required

**Request**:
```json
{
  "password": "user-password",
  "code": "123456"
}
```

**Response**: `200 OK` or `400 Bad Request`

### Get 2FA Status

**GET** `/api/v1/two-factor/status`

Returns the current 2FA status for the authenticated user.

**Authentication**: Required

**Response**:
```json
{
  "enabled": true,
  "has_backup_codes": true,
  "backup_codes_remaining": 7
}
```

### Regenerate Backup Codes

**POST** `/api/v1/two-factor/regenerate-backup-codes`

Generates a new set of backup codes (invalidates old ones).

**Authentication**: Required

**Request**:
```json
{
  "code": "123456"
}
```

**Response**:
```json
[
  "ABC-DEF-GHI",
  "JKL-MNO-PQR",
  ...
]
```

### Emergency Disable Request

**POST** `/api/v1/two-factor/emergency-disable-request`

Requests an emergency 2FA disable via email.

**Authentication**: Not required

**Request**:
```json
{
  "email": "user@example.com"
}
```

**Response**: `200 OK` (always, to prevent email enumeration)

### Emergency Disable Confirm

**POST** `/api/v1/two-factor/emergency-disable-confirm`

Confirms emergency 2FA disable with token from email.

**Authentication**: Not required

**Request**:
```json
{
  "token": "emergency-token-from-email"
}
```

**Response**: `200 OK` or `400 Bad Request`

---

## Testing

### Unit Tests

Run the 2FA test suite:

```bash
cargo test two_factor -- --ignored
```

**Note**: Tests are marked with `#[ignore]` because they require a database connection.

### Manual Testing Checklist

#### Setup Flow
- [ ] User can enable 2FA from settings
- [ ] QR code is generated and scannable
- [ ] Backup codes are displayed (10 codes)
- [ ] Verification with valid code succeeds
- [ ] Verification with invalid code fails
- [ ] 2FA status shows as enabled after verification

#### Login Flow
- [ ] User with 2FA is prompted for code after password
- [ ] Valid TOTP code allows login
- [ ] Invalid TOTP code is rejected
- [ ] Valid backup code allows login
- [ ] Used backup code cannot be reused
- [ ] Rate limiting activates after 5 failed attempts
- [ ] User can switch between TOTP and backup code input

#### Management
- [ ] Backup codes remaining count is accurate
- [ ] Regenerating backup codes works
- [ ] Old backup codes are invalidated after regeneration
- [ ] Disabling 2FA requires password and code
- [ ] 2FA status shows as disabled after disabling

#### Emergency Disable
- [ ] Emergency disable request sends email
- [ ] Request doesn't reveal if email exists (security)
- [ ] Email link disables 2FA correctly
- [ ] Token expires after 1 hour
- [ ] Used token cannot be reused

#### Security
- [ ] Secrets are encrypted in database
- [ ] Rate limiting prevents brute force
- [ ] TOTP codes expire after 30 seconds
- [ ] Emergency tokens are single-use
- [ ] Session management works correctly

### Integration Testing

Test the complete flow end-to-end:

```bash
# 1. Create a test user
curl -X POST http://localhost:8000/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Test User","email":"test@example.com","password":"SecurePass123!"}'

# 2. Login (get session cookie)
curl -X POST http://localhost:8000/api/v1/users/login \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d '{"email":"test@example.com","password":"SecurePass123!"}'

# 3. Setup 2FA
curl -X POST http://localhost:8000/api/v1/two-factor/setup \
  -H "Content-Type: application/json" \
  -b cookies.txt

# 4. Verify with TOTP code from authenticator app
curl -X POST http://localhost:8000/api/v1/two-factor/verify \
  -H "Content-Type: application/json" \
  -b cookies.txt \
  -d '{"code":"123456"}'

# 5. Check 2FA status
curl http://localhost:8000/api/v1/two-factor/status \
  -b cookies.txt
```

---

## Troubleshooting

### Common Issues

**Issue**: "Invalid two-factor authentication code"
- **Cause**: Clock drift between server and device
- **Solution**: Ensure server time is synced with NTP
- **Solution**: Check authenticator app time settings

**Issue**: "Encryption key must be exactly 32 bytes"
- **Cause**: Invalid encryption key format
- **Solution**: Use `openssl rand -hex 32` to generate valid key
- **Solution**: Ensure key is exactly 64 hexadecimal characters

**Issue**: Emergency disable email not received
- **Cause**: Email service not configured or disabled
- **Solution**: Check email configuration in `Budget.toml`
- **Solution**: Verify `email.enabled = true`
- **Solution**: Check spam/junk folder

**Issue**: QR code doesn't scan
- **Cause**: QR code rendering issues
- **Solution**: Try manual entry using the secret key
- **Solution**: Check browser console for errors
- **Solution**: Try a different authenticator app

### Debug Logging

Enable trace logging for 2FA operations:

```toml
[logging]
level = "trace"
```

Then check logs for 2FA-related messages:

```bash
# Search for 2FA operations in logs
tail -f logs/app.log | grep "2FA\|two_factor"
```

---

## Migration Guide

### Adding 2FA to Existing Application

If you're adding 2FA to an existing PiggyPulse installation:

1. **Run Database Migration**
   ```bash
   sqlx migrate run
   ```

2. **Update Configuration**
   - Generate encryption key: `openssl rand -hex 32`
   - Add `[two_factor]` section to `Budget.toml`
   - Configure email settings if not already done

3. **Test on Staging**
   - Enable 2FA on a test account
   - Verify all flows work correctly
   - Test emergency disable

4. **Deploy to Production**
   - Update configuration with production values
   - Monitor logs for errors
   - Communicate 2FA availability to users

5. **User Communication**
   - Send email announcing 2FA feature
   - Provide link to documentation
   - Emphasize optional nature (don't force enable)

### Updating Encryption Key

**WARNING**: This will require all users to re-setup 2FA!

1. Generate new key: `openssl rand -hex 32`
2. Schedule maintenance window
3. Notify users that 2FA will be reset
4. Update `encryption_key` in configuration
5. Restart application
6. Optionally: Clear all 2FA records from database
   ```sql
   DELETE FROM two_factor_auth;
   DELETE FROM two_factor_backup_codes;
   DELETE FROM two_factor_rate_limits;
   DELETE FROM two_factor_emergency_tokens;
   ```

---

## Support

For issues or questions:
- GitHub Issues: [your-repo/issues](https://github.com/your-org/your-repo/issues)
- Security Issues: security@yourapp.com (for responsible disclosure)
- Documentation: This file and inline code comments

## License

This 2FA implementation is part of PiggyPulse and follows the same license as the main project.
