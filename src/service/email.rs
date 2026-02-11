use crate::config::EmailConfig;
use crate::error::app_error::AppError;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

pub struct EmailService {
    config: EmailConfig,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    /// Send a password reset email with the reset token
    pub async fn send_password_reset_email(&self, to_email: &str, to_name: &str, reset_token: &str, reset_url: &str) -> Result<(), AppError> {
        if !self.config.enabled {
            tracing::warn!("Email service is disabled, skipping password reset email to {}", to_email);
            return Ok(());
        }

        let reset_link = format!("{}?token={}", reset_url, reset_token);

        let subject = "PiggyPulse Password Reset Request";
        let html_body = self.generate_reset_email_html(to_name, &reset_link);
        let text_body = self.generate_reset_email_text(to_name, &reset_link);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    /// Generate HTML version of password reset email
    fn generate_reset_email_html(&self, to_name: &str, reset_link: &str) -> String {
        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            background-color: #4F46E5;
            color: white;
            padding: 20px;
            text-align: center;
            border-radius: 8px 8px 0 0;
        }}
        .content {{
            background-color: #f9fafb;
            padding: 30px;
            border-radius: 0 0 8px 8px;
        }}
        .button {{
            display: inline-block;
            padding: 12px 24px;
            background-color: #4F46E5;
            color: white !important;
            text-decoration: none;
            border-radius: 6px;
            margin: 20px 0;
            font-weight: 500;
        }}
        .warning {{
            background-color: #FEF3C7;
            border-left: 4px solid #F59E0B;
            padding: 12px;
            margin: 20px 0;
            border-radius: 4px;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e5e7eb;
            font-size: 0.875rem;
            color: #6b7280;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🐷 PiggyPulse</h1>
    </div>
    <div class="content">
        <h2>Password Reset Request</h2>
        <p>Hi {},</p>
        <p>We received a request to reset your PiggyPulse password. Click the button below to create a new password:</p>

        <a href="{}" class="button">Reset Your Password</a>

        <p><strong>This link expires in 15 minutes.</strong></p>

        <div class="warning">
            <strong>⚠️ Security Tips:</strong>
            <ul style="margin: 8px 0;">
                <li>Never share this link with anyone</li>
                <li>We'll never ask for your password via email</li>
                <li>If you didn't request this, please ignore this email</li>
            </ul>
        </div>

        <p>If the button doesn't work, copy and paste this link into your browser:</p>
        <p style="word-break: break-all; font-size: 0.875rem; color: #6b7280;">{}</p>

        <div class="footer">
            <p>If you didn't request a password reset, you can safely ignore this email. Your password will remain unchanged.</p>
            <p>Best regards,<br>The PiggyPulse Security Team</p>
        </div>
    </div>
</body>
</html>
"#,
            to_name, reset_link, reset_link
        )
    }

    /// Generate plain text version of password reset email
    fn generate_reset_email_text(&self, to_name: &str, reset_link: &str) -> String {
        format!(
            r#"PiggyPulse Password Reset Request

Hi {},

We received a request to reset your PiggyPulse password.

To reset your password, visit the following link:
{}

This link expires in 15 minutes.

SECURITY TIPS:
• Never share this link with anyone
• We'll never ask for your password via email
• If you didn't request this, please ignore this email

If you didn't request a password reset, you can safely ignore this email.
Your password will remain unchanged.

Best regards,
The PiggyPulse Security Team
"#,
            to_name, reset_link
        )
    }

    /// Send an email using SMTP
    async fn send_email(&self, to_email: &str, subject: &str, html_body: &str, text_body: &str) -> Result<(), AppError> {
        // Build the email message
        let email = Message::builder()
            .from(
                format!("{} <{}>", self.config.from_name, self.config.from_address)
                    .parse()
                    .map_err(|e| AppError::email(format!("Invalid from address: {}", e)))?,
            )
            .to(to_email.parse().map_err(|e| AppError::email(format!("Invalid to address: {}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .multipart(
                lettre::message::MultiPart::alternative()
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text_body.to_string()),
                    )
                    .singlepart(
                        lettre::message::SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )
            .map_err(|e| AppError::email(format!("Failed to build email: {}", e)))?;

        // Configure SMTP transport
        let creds = Credentials::new(self.config.smtp_username.clone(), self.config.smtp_password.clone());

        let mailer = SmtpTransport::relay(&self.config.smtp_host)
            .map_err(|e| AppError::email(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(creds)
            .port(self.config.smtp_port)
            .build();

        // Send the email (blocking operation, should be run in a separate thread)
        let result = tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::email(format!("Failed to spawn email sending task: {}", e)))?;

        result.map_err(|e| AppError::email(format!("Failed to send email: {}", e)))?;

        tracing::info!("Password reset email sent successfully to {}", to_email);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_reset_email_html() {
        let config = EmailConfig {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "test".to_string(),
            smtp_password: "test".to_string(),
            from_address: "noreply@piggypulse.com".to_string(),
            from_name: "PiggyPulse".to_string(),
            enabled: false,
        };

        let service = EmailService::new(config);
        let html = service.generate_reset_email_html("John Doe", "https://example.com/reset?token=abc123");

        assert!(html.contains("John Doe"));
        assert!(html.contains("https://example.com/reset?token=abc123"));
        assert!(html.contains("Reset Your Password"));
        assert!(html.contains("15 minutes"));
        assert!(html.contains("Security Tips"));
    }

    #[test]
    fn test_generate_reset_email_text() {
        let config = EmailConfig {
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: "test".to_string(),
            smtp_password: "test".to_string(),
            from_address: "noreply@piggypulse.com".to_string(),
            from_name: "PiggyPulse".to_string(),
            enabled: false,
        };

        let service = EmailService::new(config);
        let text = service.generate_reset_email_text("Jane Smith", "https://example.com/reset?token=xyz789");

        assert!(text.contains("Jane Smith"));
        assert!(text.contains("https://example.com/reset?token=xyz789"));
        assert!(text.contains("SECURITY TIPS"));
        assert!(text.contains("15 minutes"));
    }
}
