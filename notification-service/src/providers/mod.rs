use crate::notification_models::{DeliveryChannel, Notification, NotificationResult};
use async_trait::async_trait;
use color_eyre::Result;

pub mod email;

pub use email::EmailProvider;

/// Trait for notification providers
#[async_trait]
pub trait NotificationProvider: Send + Sync {
    /// Send a notification via this provider
    async fn send(&self, notification: &Notification) -> Result<NotificationResult>;

    /// Get the supported delivery channel for this provider
    fn channel(&self) -> DeliveryChannel;

    /// Check if this provider is available/configured
    fn is_available(&self) -> bool;
}

/// Error types for notification providers
#[derive(Debug, Clone)]
pub enum NotificationError {
    SendingFailed(String),
    ServiceUnavailable,
    InvalidRecipient(String),
    RateLimited,
    ConfigurationError(String),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::SendingFailed(msg) => {
                write!(f, "Failed to send notification: {msg}")
            }
            NotificationError::ServiceUnavailable => {
                write!(f, "Notification service is temporarily unavailable")
            }
            NotificationError::InvalidRecipient(msg) => write!(f, "Invalid recipient: {msg}"),
            NotificationError::RateLimited => {
                write!(f, "Rate limited : too many notifications sent")
            }
            NotificationError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for NotificationError {}
