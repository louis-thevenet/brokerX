use super::{NotificationError, NotificationProvider};
use crate::notification_models::{
    DeliveryChannel, Notification, NotificationResult, NotificationType,
};
use async_trait::async_trait;
use color_eyre::Result;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{error, warn};

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

impl EmailConfig {
    /// Create from environment variables
    pub fn from_env() -> Result<Self, String> {
        let _ = dotenvy::dotenv();

        let smtp_server =
            std::env::var("SMTP_SERVER").unwrap_or_else(|_| "smtp.gmail.com".to_string());

        let smtp_port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .map_err(|_| "Invalid SMTP_PORT: must be a valid port number".to_string())?;

        let username = std::env::var("SMTP_USERNAME")
            .map_err(|_| "SMTP_USERNAME environment variable must be set".to_string())?;

        let password = std::env::var("SMTP_PASSWORD")
            .map_err(|_| "SMTP_PASSWORD environment variable must be set".to_string())?;

        let from_email = std::env::var("SMTP_FROM_EMAIL")
            .map_err(|_| "SMTP_FROM_EMAIL environment variable must be set".to_string())?;

        let from_name = std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "BrokerX".to_string());

        Ok(Self {
            smtp_server,
            smtp_port,
            username,
            password,
            from_email,
            from_name,
        })
    }

    /// Create a test configuration
    pub fn new_test_config() -> Self {
        Self {
            smtp_server: "test.smtp.com".to_string(),
            smtp_port: 587,
            username: "test_user".to_string(),
            password: "test_password".to_string(),
            from_email: "test@example.com".to_string(),
            from_name: "BrokerX Test".to_string(),
        }
    }
}

pub struct EmailProvider {
    config: Option<EmailConfig>,
    console_fallback: bool,
}

impl EmailProvider {
    /// Create with explicit configuration
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config: Some(config),
            console_fallback: false,
        }
    }

    /// Create with configuration from environment, fallback to console if not available
    pub fn new_with_fallback() -> Self {
        match EmailConfig::from_env() {
            Ok(config) => Self::new(config),
            Err(e) => {
                warn!(
                    "Failed to load SMTP config ({}), will use console logging fallback",
                    e
                );
                Self::new_console_fallback()
            }
        }
    }

    /// Create with console fallback only (for development)
    pub fn new_console_fallback() -> Self {
        Self {
            config: None,
            console_fallback: true,
        }
    }

    /// Create for testing
    pub fn new_for_testing() -> Self {
        Self::new(EmailConfig::new_test_config())
    }

    fn generate_email_content(
        &self,
        notification: &Notification,
    ) -> Result<(String, String), NotificationError> {
        let (subject, body) = match &notification.notification_type {
            NotificationType::OrderExecution {
                order_id,
                symbol,
                side,
                quantity,
                filled_quantity,
                price,
                status,
            } => {
                let status_text = match status.as_str() {
                    "filled" => "✅ Fully Executed",
                    "partially_filled" => "🔄 Partially Executed",
                    "rejected" => "❌ Rejected",
                    _ => "Updated",
                };

                let subject = format!(
                    "BrokerX: Order {} - {}",
                    order_id.chars().take(8).collect::<String>(),
                    status_text
                );

                let price_text = if let Some(p) = price {
                    format!("at ${p:.2}")
                } else {
                    "at market price".to_string()
                };

                let body = match status.as_str() {
                    "filled" => format!(
                        r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #d4edda; padding: 20px; text-align: center; border-radius: 8px; border: 1px solid #c3e6cb; }}
        .order-details {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .success {{ color: #155724; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header success">
        <h1>✅ Order Successfully Executed</h1>
    </div>
    
    <p>Great news! Your order has been fully executed.</p>
    
    <div class="order-details">
        <h3>Order Details</h3>
        <p><strong>Order ID:</strong> {order_id}</p>
        <p><strong>Symbol:</strong> {symbol}</p>
        <p><strong>Side:</strong> {side}</p>
        <p><strong>Quantity:</strong> {quantity} shares</p>
        <p><strong>Executed:</strong> {filled_quantity} shares</p>
        <p><strong>Price:</strong> {price_text}</p>
        <p><strong>Status:</strong> <span class="success">Fully Executed</span></p>
    </div>
    
    <p>Your portfolio has been updated to reflect this transaction.</p>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
        <p>If you have any questions, please contact our support team.</p>
    </div>
</body>
</html>
                        "#,
                        order_id = order_id,
                        symbol = symbol,
                        side = side.to_uppercase(),
                        quantity = quantity,
                        filled_quantity = filled_quantity,
                        price_text = price_text
                    ),
                    "partially_filled" => format!(
                        r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #fff3cd; padding: 20px; text-align: center; border-radius: 8px; border: 1px solid #ffeaa7; }}
        .order-details {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .warning {{ color: #856404; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header warning">
        <h1>🔄 Order Partially Executed</h1>
    </div>
    
    <p>Your order has been partially filled. We'll continue trying to fill the remaining quantity.</p>
    
    <div class="order-details">
        <h3>Order Details</h3>
        <p><strong>Order ID:</strong> {order_id}</p>
        <p><strong>Symbol:</strong> {symbol}</p>
        <p><strong>Side:</strong> {side}</p>
        <p><strong>Total Quantity:</strong> {quantity} shares</p>
        <p><strong>Filled:</strong> {filled_quantity} shares</p>
        <p><strong>Remaining:</strong> {remaining_quantity} shares</p>
        <p><strong>Price:</strong> {price_text}</p>
        <p><strong>Status:</strong> <span class="warning">Partially Filled</span></p>
    </div>
    
    <p>We'll send you another notification when the order is completely filled or if there are any issues.</p>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
        <p>If you have any questions, please contact our support team.</p>
    </div>
</body>
</html>
                        "#,
                        order_id = order_id,
                        symbol = symbol,
                        side = side.to_uppercase(),
                        quantity = quantity,
                        filled_quantity = filled_quantity,
                        remaining_quantity = quantity.saturating_sub(*filled_quantity),
                        price_text = price_text
                    ),
                    "rejected" => format!(
                        r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #f8d7da; padding: 20px; text-align: center; border-radius: 8px; border: 1px solid #f5c6cb; }}
        .order-details {{ background-color: #f8f9fa; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .danger {{ color: #721c24; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header danger">
        <h1>❌ Order Rejected</h1>
    </div>
    
    <p>Unfortunately, your order could not be executed and has been rejected.</p>
    
    <div class="order-details">
        <h3>Order Details</h3>
        <p><strong>Order ID:</strong> {order_id}</p>
        <p><strong>Symbol:</strong> {symbol}</p>
        <p><strong>Side:</strong> {side}</p>
        <p><strong>Quantity:</strong> {quantity} shares</p>
        <p><strong>Status:</strong> <span class="danger">Rejected</span></p>
    </div>
    
    <p>Common reasons for rejection include insufficient balance, market closure, or invalid order parameters.</p>
    <p>Please check your account balance and try placing the order again.</p>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
        <p>If you have any questions, please contact our support team.</p>
    </div>
</body>
</html>
                        "#,
                        order_id = order_id,
                        symbol = symbol,
                        side = side.to_uppercase(),
                        quantity = quantity
                    ),
                    _ => format!(
                        r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
                        .header {{ background-color: #f8f9fa; padding: 20px; text-align: center; border-radius: 8px; }}
        .order-details {{ background-color: #e9ecef; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>BrokerX Order Update</h1>
    </div>
    
    <p>Your order status has been updated.</p>
    
    <div class="order-details">
        <h3>Order Details</h3>
        <p><strong>Order ID:</strong> {order_id}</p>
        <p><strong>Symbol:</strong> {symbol}</p>
        <p><strong>Side:</strong> {side}</p>
        <p><strong>Quantity:</strong> {quantity} shares</p>
        <p><strong>Status:</strong> {status}</p>
    </div>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
    </div>
</body>
</html>
                        "#,
                        order_id = order_id,
                        symbol = symbol,
                        side = side.to_uppercase(),
                        quantity = quantity,
                        status = status
                    ),
                };

                (subject, body)
            }
            NotificationType::Mfa { code, .. } => {
                let subject = "BrokerX - Your Verification Code".to_string();
                let body = format!(
                    r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #f8f9fa; padding: 20px; text-align: center; border-radius: 8px; }}
        .code {{ font-size: 32px; font-weight: bold; color: #007bff; text-align: center; margin: 20px 0; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>BrokerX Security Verification</h1>
    </div>
    
    <p>Hello,</p>
    
    <p>You have requested to sign in to your BrokerX account. Please use the verification code below:</p>
    
    <div class="code">{code}</div>
    
    <p>This code will expire in 5 minutes for security reasons.</p>
    
    <p>If you did not request this code, please ignore this email.</p>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
    </div>
</body>
</html>
                    "#
                );
                (subject, body)
            }
            NotificationType::System { title, message, .. } => {
                let subject = format!("BrokerX: {title}");
                let body = format!(
                    r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #f8f9fa; padding: 20px; text-align: center; border-radius: 8px; }}
        .message {{ background-color: #e9ecef; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>BrokerX</h1>
        <h2>{title}</h2>
    </div>
    
    <div class="message">
        <p>{message}</p>
    </div>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
    </div>
</body>
</html>
                    "#
                );
                (subject, body)
            }
            NotificationType::MarketData {
                symbols,
                holdings,
                frequency,
            } => {
                let subject = format!("BrokerX: {} Market Data Report", frequency.to_uppercase());

                // Build symbols table
                let mut symbols_html = String::new();
                for sym in symbols {
                    let change_color = if sym.change >= 0.0 {
                        "#28a745"
                    } else {
                        "#dc3545"
                    };
                    let change_direction = if sym.change >= 0.0 { "↗" } else { "↘" };

                    symbols_html.push_str(&format!(
                        r#"
                        <tr>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6;"><strong>{}</strong></td>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6; color: {};">{} ${:.2} ({:.1}%)</td>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                            <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">{}</td>
                        </tr>
                        "#,
                        sym.symbol, sym.price, change_color, change_direction, sym.change, sym.change_percent, sym.high, sym.low, sym.volume
                    ));
                }

                // Build holdings table if available
                let holdings_html = if let Some(holds) = holdings {
                    let mut holdings_rows = String::new();
                    let mut total_value = 0.0;
                    let mut total_pl = 0.0;

                    for hold in holds {
                        total_value += hold.total_value;
                        total_pl += hold.profit_loss;
                        let pl_color = if hold.profit_loss >= 0.0 {
                            "#28a745"
                        } else {
                            "#dc3545"
                        };

                        holdings_rows.push_str(&format!(
                            r#"
                            <tr>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6;"><strong>{}</strong></td>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">{}</td>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6;">${:.2}</td>
                                <td style="padding: 10px; border-bottom: 1px solid #dee2e6; color: {};">${:.2} ({:.1}%)</td>
                            </tr>
                            "#,
                            hold.symbol, hold.quantity, hold.average_cost, hold.current_price, hold.total_value, pl_color, hold.profit_loss, hold.profit_loss_percent
                        ));
                    }

                    let total_pl_color = if total_pl >= 0.0 {
                        "#28a745"
                    } else {
                        "#dc3545"
                    };

                    format!(
                        r#"
                        <div style="margin-top: 30px;">
                            <h2>Your Portfolio</h2>
                            <table style="width: 100%; border-collapse: collapse; background-color: white; border-radius: 5px; overflow: hidden;">
                                <thead style="background-color: #007bff; color: white;">
                                    <tr>
                                        <th style="padding: 12px; text-align: left;">Symbol</th>
                                        <th style="padding: 12px; text-align: left;">Shares</th>
                                        <th style="padding: 12px; text-align: left;">Avg Cost</th>
                                        <th style="padding: 12px; text-align: left;">Current</th>
                                        <th style="padding: 12px; text-align: left;">Value</th>
                                        <th style="padding: 12px; text-align: left;">P&L</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {holdings_rows}
                                    <tr style="background-color: #f8f9fa; font-weight: bold;">
                                        <td colspan="4" style="padding: 12px; border-top: 2px solid #dee2e6;">TOTAL</td>
                                        <td style="padding: 12px; border-top: 2px solid #dee2e6;">${total_value:.2}</td>
                                        <td style="padding: 12px; border-top: 2px solid #dee2e6; color: {total_pl_color};">${total_pl:.2}</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                        "#
                    )
                } else {
                    String::new()
                };

                let body = format!(
                    r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; background-color: #f5f5f5; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 30px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background-color: white; padding: 30px; border-radius: 0 0 8px 8px; }}
        table {{ width: 100%; border-collapse: collapse; margin-top: 20px; background-color: white; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        th {{ background-color: #007bff; color: white; padding: 12px; text-align: left; }}
        td {{ padding: 10px; border-bottom: 1px solid #dee2e6; }}
        .footer {{ color: #666; font-size: 12px; text-align: center; margin-top: 30px; padding: 20px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>📊 {} Market Data Report</h1>
        <p style="margin: 0; opacity: 0.9;">Generated at {}</p>
    </div>
    
    <div class="content">
        <h2>Market Overview</h2>
        <table>
            <thead>
                <tr>
                    <th>Symbol</th>
                    <th>Price</th>
                    <th>Change</th>
                    <th>High</th>
                    <th>Low</th>
                    <th>Volume</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
        
        {}
        
        <p style="margin-top: 30px; color: #666; font-size: 14px;">
            This is your {} market data report. You can manage your subscription settings in your BrokerX account.
        </p>
    </div>
    
    <div class="footer">
        <p>This is an automated message from BrokerX. Please do not reply to this email.</p>
        <p>To unsubscribe from market data reports, please visit your account settings.</p>
    </div>
</body>
</html>
                    "#,
                    frequency.to_uppercase(),
                    chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"),
                    symbols_html,
                    holdings_html,
                    frequency
                );
                (subject, body)
            }
        };

        Ok((subject, body))
    }

    fn send_console_notification(
        &self,
        notification: &Notification,
    ) -> Result<(), NotificationError> {
        match &notification.notification_type {
            NotificationType::OrderExecution {
                order_id,
                symbol,
                side,
                quantity,
                filled_quantity,
                price,
                status,
            } => {
                let status_emoji = match status.as_str() {
                    "filled" => "✅",
                    "partially_filled" => "🔄",
                    "rejected" => "❌",
                    _ => "📋",
                };

                let price_text = if let Some(p) = price {
                    format!("${p:.2}")
                } else {
                    "Market".to_string()
                };

                println!("\n╔══════════════════════════════════════════════════════════╗");
                println!("║                  ORDER EXECUTION NOTIFICATION           ║");
                println!("╠══════════════════════════════════════════════════════════╣");
                println!("║  Email: {:<47} ║", notification.user_email);
                println!(
                    "║  Order ID: {:<44} ║",
                    order_id.chars().take(44).collect::<String>()
                );
                println!("║  Symbol: {symbol:<46} ║");
                println!("║  Side: {:<48} ║", side.to_uppercase());
                println!("║  Quantity: {quantity:<44} ║");
                println!("║  Filled: {filled_quantity:<46} ║");
                println!("║  Price: {price_text:<47} ║");
                println!(
                    "║  Status: {} {:<43} ║",
                    status_emoji,
                    status.replace('_', " ").to_uppercase()
                );
                println!("╚══════════════════════════════════════════════════════════╝\n");
            }
            NotificationType::Mfa { code, .. } => {
                println!("\n╔══════════════════════════════════════════╗");
                println!("║          OTP VERIFICATION CODE           ║");
                println!("╠══════════════════════════════════════════╣");
                println!("║  Email: {:<32} ║", notification.user_email);
                println!("║  Code:  {code:<32} ║");
                println!("║  Valid for: 5 minutes                    ║");
                println!("╚══════════════════════════════════════════╝\n");
            }
            _ => {
                println!(
                    "\n📧 Notification for {}: {:?} (SMTP not configured)",
                    notification.user_email, notification.notification_type
                );
            }
        }

        Ok(())
    }

    async fn send_smtp(&self, notification: &Notification) -> Result<(), NotificationError> {
        let config = self.config.as_ref().ok_or_else(|| {
            NotificationError::ConfigurationError("SMTP not configured".to_string())
        })?;

        let (subject, body) = self.generate_email_content(notification)?;

        let email = Message::builder()
            .from(
                format!("{} <{}>", config.from_name, config.from_email)
                    .parse()
                    .map_err(|e| {
                        NotificationError::SendingFailed(format!("Invalid from address: {e}"))
                    })?,
            )
            .to(notification.user_email.parse().map_err(|e| {
                NotificationError::InvalidRecipient(format!("Invalid email address: {e}"))
            })?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body)
            .map_err(|e| NotificationError::SendingFailed(format!("Failed to build email: {e}")))?;

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer = SmtpTransport::relay(&config.smtp_server)
            .map_err(|e| NotificationError::SendingFailed(format!("SMTP relay error: {e}")))?
            .credentials(creds)
            .build();

        mailer
            .send(&email)
            .map_err(|e| NotificationError::SendingFailed(format!("Failed to send email: {e}")))?;

        Ok(())
    }
}

#[async_trait]
impl NotificationProvider for EmailProvider {
    async fn send(&self, notification: &Notification) -> Result<NotificationResult> {
        // Check if MFA_BYPASS is enabled
        let mfa_bypass = std::env::var("MFA_BYPASS")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if mfa_bypass {
            match &notification.notification_type {
                NotificationType::Mfa { code, .. } => {
                    println!("EMAIL BYPASSED - MFA Code: {code}");
                }
                NotificationType::OrderExecution {
                    order_id, status, ..
                } => {
                    println!("EMAIL BYPASSED - Order {order_id} Status: {status}");
                }
                _ => {} // No logging for other types
            }

            return Ok(NotificationResult::success(DeliveryChannel::Email));
        }

        let result = if self.console_fallback && self.config.is_none() {
            self.send_console_notification(notification)
                .map_err(|e| color_eyre::eyre::eyre!("Console notification failed: {}", e))
        } else {
            self.send_smtp(notification)
                .await
                .map_err(|e| color_eyre::eyre::eyre!("SMTP notification failed: {}", e))
        };

        match result {
            Ok(()) => Ok(NotificationResult::success(DeliveryChannel::Email)),
            Err(e) => {
                error!("Email send failed for {}: {}", notification.user_email, e);
                Ok(NotificationResult::failure(
                    DeliveryChannel::Email,
                    e.to_string(),
                ))
            }
        }
    }

    fn channel(&self) -> DeliveryChannel {
        DeliveryChannel::Email
    }

    fn is_available(&self) -> bool {
        self.config.is_some() || self.console_fallback
    }
}
