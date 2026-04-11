use crate::config::EmailConfig;
use crate::error::app_error::AppError;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

pub struct EmailService {
    config: EmailConfig,
}

// ── Shared template pieces ───────────────────────────────────────────────────

/// CSS + HTML skeleton shared by every transactional email.
/// Matches the Figma "PiggyPulse — Email Templates" design system:
///   - purple-to-pink gradient header with centered brand name
///   - clean white body, Inter font stack
///   - light footer with tagline and links
fn email_shell(preheader: &str, body_html: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en" xmlns="http://www.w3.org/1999/xhtml">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="color-scheme" content="light">
    <meta name="supported-color-schemes" content="light">
    <title>PiggyPulse</title>
    <style>
        body, table, td, p, a, h1, span {{
            font-family: Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
        }}
        body {{
            margin: 0; padding: 0; width: 100% !important;
            background-color: #f4f2f7 !important; color: #1c1924 !important;
            line-height: 1.7; -webkit-text-size-adjust: 100%; -ms-text-size-adjust: 100%;
        }}
        table {{ border-collapse: collapse; mso-table-lspace: 0pt; mso-table-rspace: 0pt; }}
        td {{ border-collapse: collapse; }}
        p {{ margin: 0; }}
        a {{ color: inherit; text-decoration: none; }}
        .preheader {{
            display: none !important; visibility: hidden; opacity: 0;
            color: transparent; height: 0; width: 0; overflow: hidden; mso-hide: all;
        }}
        .wrapper {{ width: 100%; background-color: #f4f2f7; padding: 32px 16px; }}
        .card {{
            width: 100%; max-width: 560px; margin: 0 auto;
            background-color: #ffffff; border-radius: 16px; overflow: hidden;
            box-shadow: 0 4px 24px rgba(0,0,0,0.08);
        }}
        .header {{
            background: linear-gradient(171deg, #8B7EC8 0%, #C48B9F 100%);
            padding: 28px 32px 28px; text-align: center;
        }}
        .header-brand {{
            font-family: Calistoga, Georgia, serif;
            font-size: 22px; color: #ffffff; line-height: 33px;
            margin: 0;
        }}
        .body {{ padding: 32px 32px 24px; }}
        .greeting {{
            font-size: 16px; font-weight: 600; color: #1c1924;
            line-height: 27px; margin: 0 0 12px;
        }}
        .text {{
            font-size: 15px; color: #3d3a45; line-height: 25.5px;
            margin: 0 0 16px;
        }}
        .text-muted {{
            font-size: 13px; color: #7e7a90; line-height: 22px;
            margin: 0 0 12px;
        }}
        .text-faint {{
            font-size: 12px; color: #a09bac; line-height: 20px;
            margin: 0 0 12px; word-break: break-all;
        }}
        .btn-wrap {{ text-align: center; margin: 24px 0; }}
        .btn-primary {{
            display: inline-block; background-color: #8B7EC8; color: #ffffff !important;
            font-size: 14px; font-weight: 600; text-decoration: none;
            padding: 12px 28px; border-radius: 10px; line-height: 24px;
        }}
        .btn-alert {{
            display: inline-block; background-color: #C48BA0; color: #ffffff !important;
            font-size: 14px; font-weight: 600; text-decoration: none;
            padding: 12px 28px; border-radius: 10px; line-height: 24px;
        }}
        .info-box {{
            background-color: #f7f5fa; border-radius: 10px;
            padding: 14px 18px; margin: 16px 0;
        }}
        .info-row {{
            display: flex; justify-content: space-between; padding: 3px 0;
        }}
        .info-row-table {{ width: 100%; border-collapse: collapse; }}
        .info-label {{
            font-size: 13px; color: #7e7a90; line-height: 22px;
            padding: 3px 0; text-align: left;
        }}
        .info-value {{
            font-size: 13px; font-weight: 600; color: #1c1924; line-height: 22px;
            padding: 3px 0; text-align: right;
        }}
        .feature-box {{
            background-color: #f7f5fa; border-radius: 10px;
            padding: 14px 18px; margin: 16px 0;
        }}
        .feature-item {{
            font-size: 13px; color: #3d3a45; line-height: 22px;
            padding: 4px 0;
        }}
        .feature-label {{ font-weight: 700; }}
        .footer {{
            border-top: 1px solid #ede9f4; background-color: #f9f8fc;
            padding: 20px 32px 24px; text-align: center;
        }}
        .footer-brand {{
            font-family: Calistoga, Georgia, serif;
            font-size: 14px; color: #8B7EC8; margin: 0 0 6px;
        }}
        .footer-tagline {{
            font-size: 12px; color: #a09bac; margin: 0 0 8px; line-height: 18px;
        }}
        .footer-links {{
            font-size: 12px; line-height: 18px; margin: 0;
        }}
        .footer-links a {{ color: #8B7EC8; text-decoration: none; }}
        .footer-links .sep {{ color: #a09bac; }}
        @media screen and (max-width: 600px) {{
            .body {{ padding: 24px 20px 16px; }}
            .header {{ padding: 24px 20px; }}
            .footer {{ padding: 16px 20px 20px; }}
        }}
    </style>
</head>
<body>
    <div class="preheader">{preheader}</div>
    <table role="presentation" class="wrapper" width="100%" cellspacing="0" cellpadding="0" border="0">
      <tr><td align="center">
        <table role="presentation" class="card" width="560" cellspacing="0" cellpadding="0" border="0">
          <tr><td class="header">
            <p class="header-brand">PiggyPulse</p>
          </td></tr>
          <tr><td class="body">
            {body_html}
          </td></tr>
          <tr><td class="footer">
            <p class="footer-brand">PiggyPulse</p>
            <p class="footer-tagline">Your financial pulse &mdash; calm, clear, and entirely yours.</p>
            <p class="footer-links">
              <a href="https://piggy-pulse.com/help">Help &amp; Support</a>
              <span class="sep"> &middot; </span>
              <a href="https://piggy-pulse.com/privacy">Privacy Policy</a>
            </p>
          </td></tr>
        </table>
      </td></tr>
    </table>
</body>
</html>"##,
        preheader = preheader,
        body_html = body_html,
    )
}

/// Build an info-box table row (label — value).
fn info_row(label: &str, value: &str) -> String {
    format!(
        r#"<tr><td class="info-label">{label}</td><td class="info-value">{value}</td></tr>"#,
        label = label,
        value = value,
    )
}

// ── EmailService ─────────────────────────────────────────────────────────────

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    // ── 1. Welcome Email ─────────────────────────────────────────────────

    /// Send a welcome email immediately after registration.
    pub async fn send_welcome_email(&self, to_email: &str, to_name: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping welcome email to {}", to_email);
            return Ok(());
        }

        let subject = format!("Welcome to PiggyPulse, {}!", to_name);
        let html_body = self.generate_welcome_html(to_name);
        let text_body = Self::generate_welcome_text(to_name);

        self.send_email(to_email, &subject, &html_body, &text_body).await
    }

    fn generate_welcome_html(&self, to_name: &str) -> String {
        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">Welcome to PiggyPulse &mdash; your calm, non-judgmental budgeting companion. We're glad you're here.</p>
            <p class="text">Here's what PiggyPulse is about:</p>
            <div class="feature-box">
              <p class="feature-item"><span class="feature-label">No judgment.</span> We show data, not opinions. Your finances, your rules.</p>
              <p class="feature-item"><span class="feature-label">Just clarity.</span> See your reality through clean, calm data presentation.</p>
              <p class="feature-item"><span class="feature-label">Your control.</span> Customizable periods, flexible categories, your way of budgeting.</p>
            </div>
            <p class="text">Ready to set up your financial pulse? It only takes a minute.</p>
            <div class="btn-wrap">
              <a href="https://app.piggy-pulse.com" class="btn-primary">Open PiggyPulse</a>
            </div>
            <p class="text-muted">If you have any questions, we're always here to help.</p>"##,
            name = to_name,
        );
        email_shell(&format!("Welcome to PiggyPulse, {}! Your calm budgeting companion.", to_name), &body)
    }

    fn generate_welcome_text(to_name: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

Welcome to PiggyPulse — your calm, non-judgmental budgeting companion. We're glad you're here.

Here's what PiggyPulse is about:

- No judgment. We show data, not opinions. Your finances, your rules.
- Just clarity. See your reality through clean, calm data presentation.
- Your control. Customizable periods, flexible categories, your way of budgeting.

Ready to set up your financial pulse? It only takes a minute.
https://app.piggy-pulse.com

If you have any questions, we're always here to help.

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
        )
    }

    // ── 2. Password Reset ────────────────────────────────────────────────

    /// Send a password reset email with the reset token.
    pub async fn send_password_reset_email(&self, to_email: &str, to_name: &str, reset_token: &str, reset_url: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping password reset email to {}", to_email);
            return Ok(());
        }

        let reset_link = format!("{}?token={}", reset_url, reset_token);

        let subject = "Reset your password";
        let html_body = self.generate_reset_email_html(to_name, &reset_link);
        let text_body = Self::generate_reset_email_text(to_name, &reset_link);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    fn generate_reset_email_html(&self, to_name: &str, reset_link: &str) -> String {
        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">Someone (hopefully you!) requested a password reset for your PiggyPulse account.</p>
            <p class="text">Click the button below to choose a new password:</p>
            <div class="btn-wrap">
              <a href="{link}" class="btn-primary">Reset My Password</a>
            </div>
            <p class="text-muted">This link expires in 1 hour. If you didn't request this, no action is needed &mdash; your password hasn't changed.</p>
            <p class="text-faint">If the button doesn't work, copy and paste this link: {link}</p>"##,
            name = to_name,
            link = reset_link,
        );
        email_shell("Reset your PiggyPulse password. This link expires in 1 hour.", &body)
    }

    fn generate_reset_email_text(to_name: &str, reset_link: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

Someone (hopefully you!) requested a password reset for your PiggyPulse account.

Click the link below to choose a new password:
{link}

This link expires in 1 hour. If you didn't request this, no action is needed — your password hasn't changed.

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
            link = reset_link,
        )
    }

    // ── 3. Account Locked ────────────────────────────────────────────────

    /// Send an account-locked email with the unlock token link.
    pub async fn send_account_locked_email(
        &self,
        to_email: &str,
        to_name: &str,
        user_id: &str,
        unlock_token: &str,
        unlock_base_url: &str,
    ) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping account locked email to {}", to_email);
            return Ok(());
        }

        let unlock_link = format!("{}?token={}&user={}", unlock_base_url, unlock_token, user_id);

        let subject = "Your account has been temporarily locked";
        let html_body = Self::generate_account_locked_html(to_name, &unlock_link);
        let text_body = Self::generate_account_locked_text(to_name, &unlock_link);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    fn generate_account_locked_html(to_name: &str, unlock_link: &str) -> String {
        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">Your PiggyPulse account has been temporarily locked due to too many failed login attempts.</p>
            <p class="text">Click the button below to unlock your account:</p>
            <div class="btn-wrap">
              <a href="{link}" class="btn-primary">Unlock My Account</a>
            </div>
            <p class="text-muted">This link expires in 1 hour. If you didn't attempt to log in, your password may be compromised. We recommend resetting it immediately.</p>
            <p class="text-faint">If the button doesn't work, copy and paste this link: {link}</p>"##,
            name = to_name,
            link = unlock_link,
        );
        email_shell("Your PiggyPulse account has been locked due to failed login attempts.", &body)
    }

    fn generate_account_locked_text(to_name: &str, unlock_link: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

Your PiggyPulse account has been temporarily locked due to too many failed login attempts.

Unlock your account using the link below:
{link}

This link expires in 1 hour. If you didn't attempt to log in, your password may be compromised. We recommend resetting it immediately.

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
            link = unlock_link,
        )
    }

    // ── 4. Emergency 2FA Disable ─────────────────────────────────────────

    /// Send an emergency 2FA disable email with the disable token.
    pub async fn send_emergency_2fa_disable_email(&self, to_email: &str, to_name: &str, disable_token: &str, disable_url: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping emergency 2FA disable email to {}", to_email);
            return Ok(());
        }

        let disable_link = format!("{}?token={}", disable_url, disable_token);

        let subject = "Emergency Two-Factor Authentication Disable Request";
        let html_body = Self::generate_emergency_2fa_disable_html(to_name, &disable_link);
        let text_body = Self::generate_emergency_2fa_disable_text(to_name, &disable_link);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    fn generate_emergency_2fa_disable_html(to_name: &str, disable_link: &str) -> String {
        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">We received an emergency request to disable two-factor authentication on your PiggyPulse account. If you made this request, use the button below to confirm.</p>
            <div class="btn-wrap">
              <a href="{link}" class="btn-alert">Disable Two-Factor Auth</a>
            </div>
            <p class="text-muted">This link expires in 15 minutes. Disabling 2FA will make your account less secure. Only proceed if you have lost access to your authenticator device.</p>
            <p class="text">If you did NOT request this, someone may be trying to compromise your account. Secure your account immediately:</p>
            <div class="btn-wrap">
              <a href="https://piggy-pulse.com/settings/security" class="btn-alert">Secure My Account</a>
            </div>
            <p class="text-faint">If the button doesn't work, copy and paste this link: {link}</p>"##,
            name = to_name,
            link = disable_link,
        );
        email_shell(
            "Emergency request to disable two-factor authentication. This link expires in 15 minutes.",
            &body,
        )
    }

    fn generate_emergency_2fa_disable_text(to_name: &str, disable_link: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

We received an emergency request to disable two-factor authentication on your PiggyPulse account.

If you made this request, use the link below to confirm:
{link}

This link expires in 15 minutes.

WARNING:
- Disabling 2FA will make your account less secure.
- Only proceed if you have lost access to your authenticator device.
- You can re-enable 2FA anytime after regaining account access.
- If you did NOT request this, someone may be trying to compromise your account.

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
            link = disable_link,
        )
    }

    // ── 5. Security Alert — 2FA Disabled ─────────────────────────────────

    /// Send a notification after 2FA has been disabled (confirmation, not request).
    pub async fn send_2fa_disabled_email(&self, to_email: &str, to_name: &str, disabled_at: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping 2FA disabled email to {}", to_email);
            return Ok(());
        }

        let subject = "Two-factor authentication was disabled";
        let html_body = Self::generate_2fa_disabled_html(to_name, disabled_at);
        let text_body = Self::generate_2fa_disabled_text(to_name, disabled_at);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    fn generate_2fa_disabled_html(to_name: &str, disabled_at: &str) -> String {
        let rows = [info_row("When", disabled_at), info_row("Action", "2FA disabled")].join("\n");

        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">Two-factor authentication was just disabled on your PiggyPulse account.</p>
            <div class="info-box">
              <table class="info-row-table" role="presentation" cellspacing="0" cellpadding="0" border="0">
                {rows}
              </table>
            </div>
            <p class="text">If this was you, no action is needed. We recommend keeping 2FA enabled for extra security.</p>
            <p class="text">If you didn't do this, secure your account right away:</p>
            <div class="btn-wrap">
              <a href="https://piggy-pulse.com/settings/security" class="btn-alert">Secure My Account</a>
            </div>"##,
            name = to_name,
            rows = rows,
        );
        email_shell("Two-factor authentication was disabled on your PiggyPulse account.", &body)
    }

    fn generate_2fa_disabled_text(to_name: &str, disabled_at: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

Two-factor authentication was just disabled on your PiggyPulse account.

When: {disabled_at}
Action: 2FA disabled

If this was you, no action is needed. We recommend keeping 2FA enabled for extra security.

If you didn't do this, secure your account right away:
https://piggy-pulse.com/settings/security

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
            disabled_at = disabled_at,
        )
    }

    // ── 6. Security Alert — Password Changed ─────────────────────────────

    /// Send a notification after the user's password has been changed.
    pub async fn send_password_changed_email(&self, to_email: &str, to_name: &str, changed_at: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping password changed email to {}", to_email);
            return Ok(());
        }

        let subject = "Your password was changed";
        let html_body = Self::generate_password_changed_html(to_name, changed_at);
        let text_body = Self::generate_password_changed_text(to_name, changed_at);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    fn generate_password_changed_html(to_name: &str, changed_at: &str) -> String {
        let rows = info_row("When", changed_at);

        let body = format!(
            r##"<p class="greeting">Hey {name},</p>
            <p class="text">Your PiggyPulse password was just changed.</p>
            <div class="info-box">
              <table class="info-row-table" role="presentation" cellspacing="0" cellpadding="0" border="0">
                {rows}
              </table>
            </div>
            <p class="text">If this was you, you're all set. If you didn't change your password, secure your account immediately:</p>
            <div class="btn-wrap">
              <a href="https://app.piggy-pulse.com/settings" class="btn-alert">Secure My Account</a>
            </div>"##,
            name = to_name,
            rows = rows,
        );
        email_shell("Your PiggyPulse password was just changed.", &body)
    }

    fn generate_password_changed_text(to_name: &str, changed_at: &str) -> String {
        format!(
            r#"PiggyPulse

Hey {name},

Your PiggyPulse password was just changed.

When: {changed_at}

If this was you, you're all set. If you didn't change your password, secure your account immediately:
https://app.piggy-pulse.com/settings

PiggyPulse — Your financial pulse, calm, clear, and entirely yours.
"#,
            name = to_name,
            changed_at = changed_at,
        )
    }

    // ── SMTP transport ───────────────────────────────────────────────────

    async fn send_email(&self, to_email: &str, subject: &str, html_body: &str, text_body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(
                format!("{} <{}>", self.config.from_name, self.config.from_address)
                    .parse()
                    .map_err(|e| AppError::email(format!("Invalid from address: {}", e)))?,
            )
            .to(to_email.parse().map_err(|e| AppError::email(format!("Invalid to address: {}", e)))?)
            .subject(subject)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(lettre::message::SinglePart::plain(text_body.to_string()))
                    .singlepart(lettre::message::SinglePart::html(html_body.to_string())),
            )
            .map_err(|e| AppError::email(format!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(self.config.smtp_username.clone(), self.config.smtp_password.clone());

        let mailer = SmtpTransport::relay(&self.config.smtp_host)
            .map_err(|e| AppError::email(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(creds)
            .port(self.config.smtp_port)
            .build();

        let result = tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::email(format!("Failed to spawn email sending task: {}", e)))?;

        result.map_err(|e| AppError::email(format!("Failed to send email: {}", e)))?;

        tracing::info!("Email sent successfully to {}", to_email);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> EmailConfig {
        EmailConfig {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "test".to_string(),
            smtp_password: "test".to_string(),
            from_address: "noreply@piggy-pulse.com".to_string(),
            from_name: "PiggyPulse".to_string(),
            enabled: false,
        }
    }

    #[test]
    fn test_generate_welcome_html() {
        let service = EmailService::new(test_config());
        let html = service.generate_welcome_html("Leonardo");

        assert!(html.contains("Hey Leonardo,"));
        assert!(html.contains("Open PiggyPulse"));
        assert!(html.contains("No judgment."));
        assert!(html.contains("Just clarity."));
        assert!(html.contains("Your control."));
        assert!(html.contains("PiggyPulse")); // header brand
    }

    #[test]
    fn test_generate_welcome_text() {
        let text = EmailService::generate_welcome_text("Leonardo");

        assert!(text.contains("Hey Leonardo,"));
        assert!(text.contains("No judgment."));
        assert!(text.contains("piggy-pulse.com"));
    }

    #[test]
    fn test_generate_reset_email_html() {
        let service = EmailService::new(test_config());
        let html = service.generate_reset_email_html("John Doe", "https://example.com/reset?token=abc123");

        assert!(html.contains("John Doe"));
        assert!(html.contains("https://example.com/reset?token=abc123"));
        assert!(html.contains("Reset My Password"));
        assert!(html.contains("1 hour"));
    }

    #[test]
    fn test_generate_reset_email_text() {
        let text = EmailService::generate_reset_email_text("Jane Smith", "https://example.com/reset?token=xyz789");

        assert!(text.contains("Jane Smith"));
        assert!(text.contains("https://example.com/reset?token=xyz789"));
        assert!(text.contains("1 hour"));
    }

    #[test]
    fn test_generate_account_locked_html() {
        let html = EmailService::generate_account_locked_html("John Doe", "https://example.com/unlock?token=abc123");

        assert!(html.contains("John Doe"));
        assert!(html.contains("https://example.com/unlock?token=abc123"));
        assert!(html.contains("Unlock My Account"));
        assert!(html.contains("1 hour"));
        assert!(html.contains("too many failed login attempts"));
    }

    #[test]
    fn test_generate_account_locked_text() {
        let text = EmailService::generate_account_locked_text("Jane Smith", "https://example.com/unlock?token=xyz789");

        assert!(text.contains("Jane Smith"));
        assert!(text.contains("https://example.com/unlock?token=xyz789"));
        assert!(text.contains("too many failed login attempts"));
    }

    #[test]
    fn test_generate_emergency_2fa_disable_html() {
        let html = EmailService::generate_emergency_2fa_disable_html("John Doe", "https://example.com/2fa/emergency?token=abc123");

        assert!(html.contains("John Doe"));
        assert!(html.contains("https://example.com/2fa/emergency?token=abc123"));
        assert!(html.contains("Disable Two-Factor Auth"));
        assert!(html.contains("15 minutes"));
    }

    #[test]
    fn test_generate_emergency_2fa_disable_text() {
        let text = EmailService::generate_emergency_2fa_disable_text("Jane Smith", "https://example.com/2fa/emergency?token=xyz789");

        assert!(text.contains("Jane Smith"));
        assert!(text.contains("https://example.com/2fa/emergency?token=xyz789"));
        assert!(text.contains("15 minutes"));
    }

    #[test]
    fn test_generate_2fa_disabled_html() {
        let html = EmailService::generate_2fa_disabled_html("Leonardo", "Apr 12, 2026 at 3:12 PM");

        assert!(html.contains("Hey Leonardo,"));
        assert!(html.contains("Two-factor authentication was just disabled"));
        assert!(html.contains("Apr 12, 2026 at 3:12 PM"));
        assert!(html.contains("2FA disabled"));
        assert!(html.contains("Secure My Account"));
    }

    #[test]
    fn test_generate_2fa_disabled_text() {
        let text = EmailService::generate_2fa_disabled_text("Leonardo", "Apr 12, 2026 at 3:12 PM");

        assert!(text.contains("Hey Leonardo,"));
        assert!(text.contains("Apr 12, 2026 at 3:12 PM"));
        assert!(text.contains("2FA disabled"));
    }

    #[test]
    fn test_generate_password_changed_html() {
        let html = EmailService::generate_password_changed_html("Leonardo", "Apr 12, 2026 at 4:05 PM");

        assert!(html.contains("Hey Leonardo,"));
        assert!(html.contains("password was just changed"));
        assert!(html.contains("Apr 12, 2026 at 4:05 PM"));
        assert!(html.contains("Secure My Account"));
    }

    #[test]
    fn test_generate_password_changed_text() {
        let text = EmailService::generate_password_changed_text("Leonardo", "Apr 12, 2026 at 4:05 PM");

        assert!(text.contains("Hey Leonardo,"));
        assert!(text.contains("Apr 12, 2026 at 4:05 PM"));
        assert!(text.contains("app.piggy-pulse.com/settings"));
    }

    #[test]
    fn test_email_shell_structure() {
        let html = email_shell("Test preheader", "<p>Test body</p>");

        assert!(html.contains("Test preheader"));
        assert!(html.contains("Test body"));
        assert!(html.contains("PiggyPulse")); // header
        assert!(html.contains("calm, clear, and entirely yours")); // footer tagline
        assert!(html.contains("Help &amp; Support"));
        assert!(html.contains("Privacy Policy"));
        assert!(html.contains("#8B7EC8")); // brand purple
        assert!(html.contains("#C48B9F")); // gradient pink
    }
}
