use crate::notification_models::Notification;
use crate::service::NotificationService;
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType};
use color_eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// User information needed for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub firstname: String,
    pub surname: String,
}

pub struct NotificationEventConsumer {
    notification_service: Arc<NotificationService>,
    user_service_url: String,
    redis_url: String,
    http_client: Client,
}

impl NotificationEventConsumer {
    pub fn new(
        notification_service: Arc<NotificationService>,
        user_service_url: String,
        redis_url: String,
    ) -> Self {
        Self {
            notification_service,
            user_service_url,
            redis_url,
            http_client: Client::new(),
        }
    }

    /// Start consuming notification events
    pub async fn start(self) -> Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "notification_events");

        info!("Notification Event Consumer starting - listening for order execution events");

        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let consumer = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = consumer.handle_event(event).await {
                        error!("Error handling notification event: {}", e);
                    }
                });
                Ok(())
            })
            .await
    }

    /// Handle individual order events - SIMPLIFIED LOGIC
    /// Each event type maps directly to a notification status
    async fn handle_event(&self, event: OrderEventPayload) -> Result<()> {
        info!(
            "Notification event: order={}, type={:?}, user={}",
            event.order_id, event.event_type, event.user_id
        );

        // Map event types to notification statuses
        let status = match event.event_type {
            OrderEventType::OrderExecuted | OrderEventType::OrderCompleted => "filled",
            OrderEventType::OrderPartiallyFilled => "partially_filled",
            OrderEventType::OrderRejected | OrderEventType::OrderFailed => "rejected",
            OrderEventType::OrderCancelled => "cancelled",
            _ => {
                // Skip events that don't need notifications
                info!("Skipping notification for {:?}", event.event_type);
                return Ok(());
            }
        };

        info!(
            "Sending {} notification for order {}",
            status, event.order_id
        );
        self.send_order_notification(&event, status).await
    }

    /// Send order notification  
    async fn send_order_notification(&self, event: &OrderEventPayload, status: &str) -> Result<()> {
        let user_info = self.get_user_info(&event.user_id).await?;

        // Determine filled quantity based on event type
        let filled_quantity = if status == "filled" {
            event
                .filled_quantity
                .map_or(event.quantity.unsigned_abs(), i64::unsigned_abs)
        } else if status == "partially_filled" {
            event.filled_quantity.unwrap_or(0).unsigned_abs()
        } else {
            0
        };

        // Convert order side to string
        let side = match event.order_side {
            brokerx_events::OrderSide::Buy => "buy",
            brokerx_events::OrderSide::Sell => "sell",
        };

        let price = match &event.order_type {
            brokerx_events::OrderType::Limit { price } => Some(*price),
            brokerx_events::OrderType::Market => event.average_price,
        };

        // Create and send notification
        let notification = Notification::order_execution(
            user_info.id,
            user_info.email,
            event.order_id.clone(),
            event.symbol.clone(),
            side.to_string(),
            event.quantity.unsigned_abs(),
            filled_quantity,
            price,
            status.to_string(),
        );

        let results = self
            .notification_service
            .send_notification(notification)
            .await?;

        for result in results {
            if result.success {
                info!(
                    "{} notification sent via {:?} for order {}",
                    status, result.channel, event.order_id
                );
            } else {
                error!(
                    "Notification failed via {:?} for order {}: {:?}",
                    result.channel, event.order_id, result.error
                );
            }
        }

        Ok(())
    }

    /// Get user information from user service
    async fn get_user_info(&self, user_id: &str) -> Result<UserInfo> {
        let url = format!("{}/v1/users/{}", self.user_service_url, user_id);

        let response = self
            .http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to get user info: HTTP {}",
                response.status()
            ));
        }

        let user_info: UserInfo = response.json().await?;
        Ok(user_info)
    }
}

impl Clone for NotificationEventConsumer {
    fn clone(&self) -> Self {
        Self {
            notification_service: self.notification_service.clone(),
            user_service_url: self.user_service_url.clone(),
            redis_url: self.redis_url.clone(),
            http_client: self.http_client.clone(),
        }
    }
}
