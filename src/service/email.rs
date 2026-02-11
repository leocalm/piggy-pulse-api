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

        let subject = "Reset your PiggyPulse password";
        let html_body = self.generate_reset_email_html(to_name, &reset_link);
        let text_body = self.generate_reset_email_text(to_name, &reset_link);

        self.send_email(to_email, subject, &html_body, &text_body).await
    }

    /// Generate HTML version of password reset email
    fn generate_reset_email_html(&self, to_name: &str, reset_link: &str) -> String {
        format!(
            r##"
<!DOCTYPE html>
<html lang="en" xmlns="http://www.w3.org/1999/xhtml" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:w="urn:schemas-microsoft-com:office:word">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="color-scheme" content="light">
    <meta name="supported-color-schemes" content="light">
    <title>Reset your PiggyPulse password</title>
    <!--[if mso]>
    <xml>
        <o:OfficeDocumentSettings>
            <o:PixelsPerInch>96</o:PixelsPerInch>
        </o:OfficeDocumentSettings>
    </xml>
    <![endif]-->
    <style>
        body, table, td, p, a, h1 {{
            font-family: Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
        }}

        body {{
            margin: 0;
            padding: 0;
            width: 100% !important;
            background-color: #FAFBFC !important;
            color: #141517 !important;
            line-height: 1.6;
            -webkit-text-size-adjust: 100%;
            -ms-text-size-adjust: 100%;
        }}

        table {{
            border-collapse: collapse;
            mso-table-lspace: 0pt;
            mso-table-rspace: 0pt;
        }}

        td {{
            border-collapse: collapse;
        }}

        img {{
            border: 0;
            outline: none;
            text-decoration: none;
            -ms-interpolation-mode: bicubic;
            display: block;
        }}

        p {{
            margin: 0;
        }}

        a {{
            color: inherit;
        }}

        a[x-apple-data-detectors],
        u + #body a,
        #MessageViewBody a {{
            color: inherit !important;
            text-decoration: none !important;
            font: inherit !important;
            line-height: inherit !important;
        }}

        .preheader {{
            display: none !important;
            visibility: hidden;
            opacity: 0;
            color: transparent;
            height: 0;
            width: 0;
            overflow: hidden;
            mso-hide: all;
        }}

        .wrapper {{
            width: 100%;
            background-color: #FAFBFC;
            padding: 28px 12px;
        }}

        .card {{
            width: 100%;
            max-width: 640px;
            margin: 0 auto;
            background-color: #FFFFFF;
            border: 1px solid rgba(0, 0, 0, 0.08);
            border-radius: 16px;
            overflow: hidden;
            box-shadow: 0 8px 24px rgba(20, 21, 23, 0.08);
        }}

        .brand-header {{
            background-color: #FFFFFF;
            border-bottom: 1px solid rgba(0, 0, 0, 0.08);
            padding: 24px 24px 18px;
        }}

        .brand-row {{
            display: table;
            width: 100%;
        }}

        .brand-logo-cell {{
            display: table-cell;
            width: 52px;
            vertical-align: middle;
        }}

        .brand-title-cell {{
            display: table-cell;
            vertical-align: middle;
            padding-left: 12px;
        }}

        .brand-title {{
            margin: 0;
            color: #4FD1FF !important;
            display: inline-block;
            font-size: 26px;
            font-weight: 700;
            letter-spacing: -0.01em;
            line-height: 1.2;
            /* Match app brand gradient tokens exactly */
            background: linear-gradient(90deg, #4FD1FF, #9B6BFF);
            background-clip: text;
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}

        .brand-subtitle {{
            margin: 4px 0 0;
            color: #2E3035 !important;
            font-size: 13px;
            font-weight: 500;
            letter-spacing: 0.01em;
        }}

        .content {{
            padding: 28px 24px 16px;
            color: #141517 !important;
            background-color: #FFFFFF;
        }}

        .eyebrow {{
            margin: 0 0 8px;
            color: #5C5F66 !important;
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.08em;
            font-weight: 700;
        }}

        .title {{
            margin: 0 0 14px;
            color: #141517 !important;
            font-size: 28px;
            line-height: 1.2;
            letter-spacing: -0.02em;
            font-weight: 700;
        }}

        .body-text {{
            margin: 0 0 14px;
            color: #2E3035 !important;
            font-size: 15px;
        }}

        .cta-wrap {{
            margin: 24px 0 20px;
        }}

        .button {{
            display: inline-block;
            /* Match app action button gradient */
            background-color: #00D4FF;
            background-image: linear-gradient(135deg, #00D4FF 0%, #B47AFF 100%);
            color: #FFFFFF !important;
            text-decoration: none;
            font-size: 15px;
            font-weight: 700;
            line-height: 1;
            border-radius: 12px;
            padding: 14px 22px;
            border: 0;
            mso-padding-alt: 0;
        }}

        .meta {{
            margin: 0 0 20px;
            color: #5C5F66 !important;
            font-size: 13px;
            font-weight: 600;
        }}

        .warning {{
            margin: 0 0 20px;
            padding: 14px 16px;
            background-color: #F8F9FA;
            border: 1px solid rgba(0, 0, 0, 0.1);
            border-left: 4px solid #FFA940;
            border-radius: 12px;
        }}

        .warning-title {{
            margin: 0 0 8px;
            color: #141517 !important;
            font-size: 14px;
            font-weight: 700;
        }}

        .warning-list {{
            margin: 0;
            padding: 0 0 0 18px;
            color: #2E3035 !important;
            font-size: 14px;
        }}

        .warning-list li {{
            margin: 0 0 6px;
        }}

        .link-box {{
            margin: 0 0 18px;
            padding: 12px 14px;
            background-color: #F1F3F5;
            border: 1px solid rgba(0, 0, 0, 0.08);
            border-radius: 12px;
            color: #5C5F66 !important;
            font-size: 12px;
            line-height: 1.5;
            word-break: break-all;
        }}

        .footer {{
            border-top: 1px solid rgba(0, 0, 0, 0.08);
            padding: 18px 24px 24px;
            background-color: #FAFBFC;
        }}

        .footer-text {{
            margin: 0 0 10px;
            color: #5C5F66 !important;
            font-size: 12px;
        }}

        .footer-signoff {{
            margin: 0;
            color: #2E3035 !important;
            font-size: 12px;
            font-weight: 600;
        }}

        @media screen and (max-width: 640px) {{
            .title {{
                font-size: 24px;
            }}

            .content {{
                padding: 22px 18px 14px;
            }}

            .brand-header {{
                padding: 20px 18px 16px;
            }}

            .footer {{
                padding: 16px 18px 20px;
            }}
        }}
    </style>
</head>
<body id="body">
    <div class="preheader">Reset your PiggyPulse password. This secure link expires in 15 minutes.</div>
    <table role="presentation" class="wrapper" width="100%" cellspacing="0" cellpadding="0" border="0">
      <tr>
        <td align="center">
          <table role="presentation" class="card" width="640" cellspacing="0" cellpadding="0" border="0">
            <tr>
              <td class="brand-header">
                <div class="brand-row">
                  <div class="brand-logo-cell">
                   <svg width="64" height="64" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path fill-rule="evenodd" clip-rule="evenodd" d="M49.2005 11.1843C51.9738 11.0523 51.6388 11.3719 51.4213 13.3509C51.1955 15.4067 49.7864 18.5138 49.1464 20.2106C50.8648 21.6925 52.0914 23.2564 52.9534 24.7257C52.8799 24.5204 52.8786 24.4944 52.9844 24.6831C53.9623 26.4274 54.375 26.3508 54.9653 26.8266C55.9159 27.3281 56.334 27.2221 57.2905 27.4611C57.8193 27.5935 58.9044 28.0413 59.2482 29.2292C59.5918 30.4189 59.7503 35.4428 59.4069 36.4216C59.0635 37.399 58.7343 38.4302 57.484 38.6811C55.4163 39.0946 53.7845 40.2306 53.1043 40.8825C51.9132 43.1355 50.0039 45.1889 46.9913 46.5506C46.6893 46.6871 46.3759 46.8144 46.0589 46.9375C46.1821 47.622 45.7946 50.2502 45.6643 50.7987C45.3168 52.2582 45.217 52.7367 43.9348 52.8416C41.9628 52.8067 39.8494 52.88 37.4156 52.7642C36.8662 52.7378 36.3688 52.2139 36.1853 51.6383C36.1853 51.6383 35.2586 49.4925 35.5237 48.9068C31.355 49.1356 27.0517 48.9148 23.3287 48.5547C23.1935 49.347 22.8027 51.0026 22.5704 51.5068C22.3191 52.0512 22.2277 52.7122 20.9725 52.7913C16.3119 52.7389 18.2989 52.7888 14.9485 52.6055C14.4752 52.5792 13.7549 51.8726 13.4318 51.2707C13.4318 51.2707 11.737 46.9604 11.8378 46.4036C9.81635 45.1003 7.21092 41.9874 6.37482 36.1779C9.45713 36.2812 19.4193 36.2708 19.4713 36.2707L21.0498 33.9029L23.6807 40.2171C24.001 40.8412 24.9968 41.7936 25.8938 40.248C26.8154 38.659 28.6432 31.2776 29.3604 27.7745C30.2941 31.5297 31.4154 36.0096 32.3937 39.5671C32.7345 40.804 34.0497 41.1286 34.746 39.8302C35.9517 37.5796 36.4846 35.6208 37.0055 34.4136L38.7388 36.2862C40.9002 36.3017 43.4281 36.3326 45.5946 36.3017C47.7778 36.2705 47.4053 34.0267 45.5018 34.0267C44.449 34.0267 41.5192 34.0938 39.8376 34.0886C39.327 33.48 39.5865 33.7244 38.2436 32.1232C37.8412 31.6434 36.5877 29.8637 35.5662 32.0303C34.6685 33.9348 34.096 35.6312 33.8484 36.2088C32.8585 32.3212 30.8627 24.3708 30.6759 23.6888C30.1497 21.7698 28.5711 21.9555 28.1533 23.565C27.7277 25.204 25.6614 32.8359 24.6557 36.441C24.1452 35.2652 23.0716 32.6905 22.7367 31.8446C22.1486 30.3589 21.251 29.6316 19.8272 31.8137C18.8971 33.2393 18.455 33.9545 18.2951 34.0577C14.5426 34.0577 10.4684 34.0268 6.23166 34.0267C6.33674 32.1294 6.95786 28.3932 8.17002 25.2248C6.95257 24.7816 5.29612 23.4705 4.94716 22.4933C4.57499 21.4493 4.02142 20.8977 4.7808 18.5934C5.51825 16.3591 8.09459 15.9811 8.24353 15.9509C9.13322 15.7625 9.56073 15.7592 10.0581 15.8116C11.3126 16.0224 11.3609 17.6324 10.4372 17.8854C9.86191 18.0423 9.91805 18.1197 8.39829 18.2645C7.2985 18.3693 6.96485 20.4558 7.30337 21.054C7.77338 21.8838 8.46887 22.4401 9.54738 22.385C9.91784 21.7988 10.3216 21.2846 10.7584 20.8722C16.5774 15.4915 27.0117 11.5439 39.0135 14.9952C40.4501 13.6777 44.153 11.4246 49.2005 11.1843ZM47.5214 26.5905C46.709 26.726 45.9631 27.232 45.8616 28.8229C45.8616 30.2785 46.4953 31.1939 47.7922 31.1598C49.0777 31.1253 49.4313 29.8028 49.3824 28.6527C49.3143 27.063 48.3891 26.4464 47.5214 26.5905Z" fill="url(#paint0_linear_35_88)"/>
                    <defs>
                    <linearGradient id="paint0_linear_35_88" x1="8.13283" y1="16.3671" x2="50.9438" y2="44.0181" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#38BDF8"/>
                    <stop offset="0.657757" stop-color="#6366F1"/>
                    <stop offset="1" stop-color="#D946EF"/>
                    </linearGradient>
                    </defs>
                    </svg>
                  </div>
                  <div class="brand-title-cell">
                    <p class="brand-title">PiggyPulse</p>
                    <p class="brand-subtitle">Secure Account Access</p>
                  </div>
                </div>
              </td>
            </tr>
            <tr>
              <td class="content">
                <p class="eyebrow">Password Reset</p>
                <h1 class="title">Reset your password</h1>
                <p class="body-text">Hi {},</p>
                <p class="body-text">We received a request to reset your PiggyPulse password. Use the button below to set a new one.</p>

                <div class="cta-wrap">
                  <!--[if mso]>
                  <v:roundrect xmlns:v="urn:schemas-microsoft-com:vml" href="{}" style="height:44px;v-text-anchor:middle;width:240px;" arcsize="18%" stroke="f" fillcolor="#00D4FF">
                    <v:fill type="gradient" color="#00D4FF" color2="#B47AFF" angle="315"/>
                    <w:anchorlock/>
                    <center style="color:#FFFFFF;font-family:Segoe UI, Arial, sans-serif;font-size:15px;font-weight:700;">
                      Reset Your Password
                    </center>
                  </v:roundrect>
                  <![endif]-->
                  <!--[if !mso]><!-- -->
                  <a href="{}" class="button">Reset Your Password</a>
                  <!--<![endif]-->
                </div>

                <p class="meta">This secure link expires in 15 minutes.</p>

                <div class="warning">
                  <p class="warning-title">Security checklist</p>
                  <ul class="warning-list">
                    <li>Never share this link with anyone.</li>
                    <li>PiggyPulse will never ask for your password by email.</li>
                    <li>If you did not request this, you can safely ignore this message.</li>
                  </ul>
                </div>

                <p class="body-text">If the button does not open, copy and paste this URL into your browser:</p>
                <p class="link-box">{}</p>
              </td>
            </tr>
            <tr>
              <td class="footer">
                <p class="footer-text">If you did not request a password reset, no action is required and your current password stays active.</p>
                <p class="footer-signoff">PiggyPulse Security</p>
              </td>
            </tr>
          </table>
        </td>
      </tr>
    </table>
</body>
</html>
"##,
            to_name, reset_link, reset_link, reset_link
        )
    }

    /// Generate plain text version of password reset email
    fn generate_reset_email_text(&self, to_name: &str, reset_link: &str) -> String {
        format!(
            r#"PiggyPulse | Password Reset

Hi {},

We received a request to reset your PiggyPulse password.

Reset your password using the secure link below:
{}

This secure link expires in 15 minutes.

Security checklist:
- Never share this link with anyone.
- PiggyPulse will never ask for your password by email.
- If you did not request this, you can safely ignore this message.

If you did not request a password reset, no action is required and your current password stays active.

PiggyPulse Security
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
        assert!(html.contains("Security checklist"));
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
        assert!(text.contains("Security checklist"));
        assert!(text.contains("15 minutes"));
    }
}
