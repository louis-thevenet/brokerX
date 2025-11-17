use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of notifications that can be sent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationType {
    /// Order execution notifications
    OrderExecution {
        order_id: String,
        symbol: String,
        side: String, // "buy" or "sell"
        quantity: u64,
        filled_quantity: u64,
        price: Option<f64>,
        status: String, // "filled", "partially_filled", "rejected"
    },
    /// MFA/OTP notifications (backward compatibility)
    Mfa { challenge_id: String, code: String },
    /// Market data notifications
    MarketData {
        symbols: Vec<MarketDataSymbol>,
        holdings: Option<Vec<HoldingData>>,
        frequency: String, // "hourly", "daily", "weekly"
    },
    /// General system notifications
    System {
        title: String,
        message: String,
        priority: Priority,
    },
}

/// Market data for a specific symbol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarketDataSymbol {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub high: f64,
    pub low: f64,
    pub volume: i64,
}

/// Holding data for portfolio summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HoldingData {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
    pub current_price: f64,
    pub total_value: f64,
    pub profit_loss: f64,
    pub profit_loss_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification delivery channels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryChannel {
    Email,
    // Future channels can be added here: Sms, Push, etc.
}

/// A notification to be sent to a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub user_email: String,
    pub notification_type: NotificationType,
    pub channels: Vec<DeliveryChannel>,
    pub created_at: DateTime<Utc>,
    pub attempts: u32,
    pub max_attempts: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
}

impl Notification {
    /// Create a new notification
    pub fn new(
        user_id: String,
        user_email: String,
        notification_type: NotificationType,
        channels: Vec<DeliveryChannel>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            user_email,
            notification_type,
            channels,
            created_at: Utc::now(),
            attempts: 0,
            max_attempts: 3,
            next_retry_at: None,
        }
    }

    /// Create an order execution notification
    pub fn order_execution(
        user_id: String,
        user_email: String,
        order_id: String,
        symbol: String,
        side: String,
        quantity: u64,
        filled_quantity: u64,
        price: Option<f64>,
        status: String,
    ) -> Self {
        let notification_type = NotificationType::OrderExecution {
            order_id,
            symbol,
            side,
            quantity,
            filled_quantity,
            price,
            status,
        };

        Self::new(
            user_id,
            user_email,
            notification_type,
            vec![DeliveryChannel::Email],
        )
    }

    /// Create an MFA notification (for backward compatibility)
    pub fn mfa(user_id: String, user_email: String, challenge_id: String, code: String) -> Self {
        let notification_type = NotificationType::Mfa { challenge_id, code };

        Self::new(
            user_id,
            user_email,
            notification_type,
            vec![DeliveryChannel::Email],
        )
    }

    /// Create a market data report notification
    pub fn market_data_report(
        user_id: String,
        user_email: String,
        symbols: Vec<MarketDataSymbol>,
        holdings: Option<Vec<HoldingData>>,
        frequency: String,
    ) -> Self {
        let notification_type = NotificationType::MarketData {
            symbols,
            holdings,
            frequency,
        };

        Self::new(
            user_id,
            user_email,
            notification_type,
            vec![DeliveryChannel::Email],
        )
    }

    /// Check if the notification can be retried
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
    }

    /// Mark an attempt
    pub fn mark_attempt(&mut self) {
        self.attempts += 1;
        if self.can_retry() {
            // Exponential backoff: 1min, 2min, 4min
            let backoff_minutes = 2_u32.pow(self.attempts - 1);
            self.next_retry_at =
                Some(Utc::now() + chrono::Duration::minutes(backoff_minutes as i64));
        } else {
            self.next_retry_at = None;
        }
    }
}

/// Result of a notification delivery attempt
#[derive(Debug, Clone)]
pub struct NotificationResult {
    pub success: bool,
    pub channel: DeliveryChannel,
    pub error: Option<String>,
    pub retry_after: Option<DateTime<Utc>>,
}

impl NotificationResult {
    pub fn success(channel: DeliveryChannel) -> Self {
        Self {
            success: true,
            channel,
            error: None,
            retry_after: None,
        }
    }

    pub fn failure(channel: DeliveryChannel, error: String) -> Self {
        Self {
            success: false,
            channel,
            error: Some(error),
            retry_after: None,
        }
    }

    pub fn failure_with_retry(
        channel: DeliveryChannel,
        error: String,
        retry_after: DateTime<Utc>,
    ) -> Self {
        Self {
            success: false,
            channel,
            error: Some(error),
            retry_after: Some(retry_after),
        }
    }
}
