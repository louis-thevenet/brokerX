use crate::notification_models::Notification;
use crate::service::NotificationService;
use color_eyre::Result;
use futures_util::StreamExt;
use redis::Client;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct DirectNotificationConsumer {
    notification_service: Arc<NotificationService>,
    redis_client: Client,
}

impl DirectNotificationConsumer {
    pub fn new(notification_service: Arc<NotificationService>, redis_url: String) -> Result<Self> {
        let redis_client = Client::open(redis_url)?;
        Ok(Self {
            notification_service,
            redis_client,
        })
    }

    /// Start consuming direct notifications from Redis pub/sub
    pub async fn start(self) -> Result<()> {
        #[allow(deprecated)]
        let conn = self.redis_client.get_async_connection().await?;
        let mut pubsub = conn.into_pubsub();

        info!("Subscribing to 'notifications' Redis channel for direct notifications");
        pubsub.subscribe("notifications").await?;

        loop {
            match pubsub.on_message().next().await {
                Some(msg) => {
                    let payload: String = match msg.get_payload() {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Failed to get message payload: {}", e);
                            continue;
                        }
                    };

                    match serde_json::from_str::<Notification>(&payload) {
                        Ok(notification) => {
                            info!(
                                "Received direct notification for user {}: {:?}",
                                notification.user_id, notification.notification_type
                            );

                            // Send the notification
                            if let Err(e) = self
                                .notification_service
                                .send_notification(notification)
                                .await
                            {
                                error!("Failed to send direct notification: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse notification JSON: {}", e);
                        }
                    }
                }
                None => {
                    warn!("Redis pub/sub connection closed, reconnecting...");
                    break;
                }
            }
        }

        Ok(())
    }
}
