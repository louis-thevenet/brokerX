// Re-export notification functionality
pub mod consumer;
pub mod direct_notification_consumer;
pub mod notification_models;
pub mod providers;
pub mod service;

pub use consumer::NotificationEventConsumer;
pub use direct_notification_consumer::DirectNotificationConsumer;
pub use notification_models::*;
pub use providers::{EmailProvider, NotificationProvider};
pub use service::NotificationService;

// Old MFA module
pub mod mfa;
pub use mfa::*;
