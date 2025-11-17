use crate::notification_models::{DeliveryChannel, Notification, NotificationResult};
use crate::providers::{EmailProvider, NotificationProvider};
use color_eyre::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct NotificationService {
    providers: HashMap<DeliveryChannel, Arc<dyn NotificationProvider>>,
    retry_queue: Arc<RwLock<Vec<Notification>>>,
}

impl NotificationService {
    pub fn new() -> Self {
        let mut providers: HashMap<DeliveryChannel, Arc<dyn NotificationProvider>> = HashMap::new();

        // Register email provider
        let email_provider = Arc::new(EmailProvider::new_with_fallback());
        providers.insert(DeliveryChannel::Email, email_provider);

        Self {
            providers,
            retry_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with custom providers
    pub fn with_providers(
        providers: HashMap<DeliveryChannel, Arc<dyn NotificationProvider>>,
    ) -> Self {
        Self {
            providers,
            retry_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Send a notification through all specified channels
    pub async fn send_notification(
        &self,
        mut notification: Notification,
    ) -> Result<Vec<NotificationResult>> {
        info!(
            "Sending notification {} to user {} via channels: {:?}",
            notification.id, notification.user_id, notification.channels
        );

        let mut results = Vec::new();
        let mut has_success = false;

        for channel in &notification.channels {
            if let Some(provider) = self.providers.get(channel) {
                if !provider.is_available() {
                    warn!("Provider for {:?} is not available", channel);
                    results.push(NotificationResult::failure(
                        channel.clone(),
                        "Provider not available".to_string(),
                    ));
                    continue;
                }

                match provider.send(&notification).await {
                    Ok(result) => {
                        if result.success {
                            has_success = true;
                            info!("Notification sent successfully via {:?}", channel);
                        } else {
                            warn!("Notification failed via {:?}: {:?}", channel, result.error);
                        }
                        results.push(result);
                    }
                    Err(e) => {
                        error!("Provider error for {:?}: {}", channel, e);
                        results.push(NotificationResult::failure(channel.clone(), e.to_string()));
                    }
                }
            } else {
                warn!("No provider registered for channel {:?}", channel);
                results.push(NotificationResult::failure(
                    channel.clone(),
                    "No provider registered".to_string(),
                ));
            }
        }

        // If no channel succeeded and notification can be retried, queue it
        if !has_success && notification.can_retry() {
            notification.mark_attempt();
            info!(
                "Notification {} failed on all channels, queuing for retry (attempt {}/{})",
                notification.id, notification.attempts, notification.max_attempts
            );
            self.queue_for_retry(notification).await;
        }

        Ok(results)
    }

    /// Process retry queue
    pub async fn process_retries(&self) -> Result<()> {
        let now = chrono::Utc::now();

        // First, collect notifications that need retry
        let notifications_to_retry = {
            let retry_queue = self.retry_queue.read().await;
            let mut notifications = Vec::new();

            for notification in retry_queue.iter() {
                if let Some(retry_time) = notification.next_retry_at {
                    if now >= retry_time {
                        notifications.push(notification.clone());
                    }
                }
            }
            notifications
        };

        // Process retries (without holding the lock)
        for notification in notifications_to_retry {
            info!("Processing retry for notification {}", notification.id);
            let _results = self.send_notification(notification).await;
        }

        // Remove processed notifications from queue
        {
            let mut retry_queue = self.retry_queue.write().await;
            retry_queue.retain(|notification| {
                if let Some(retry_time) = notification.next_retry_at {
                    now < retry_time // Keep notifications that still need retry
                } else {
                    false // Remove notifications that don't need retry anymore
                }
            });
        }

        Ok(())
    }

    /// Add notification to retry queue
    async fn queue_for_retry(&self, notification: Notification) {
        let mut retry_queue = self.retry_queue.write().await;
        retry_queue.push(notification);
    }

    /// Get retry queue size (for monitoring)
    pub async fn retry_queue_size(&self) -> usize {
        let retry_queue = self.retry_queue.read().await;
        retry_queue.len()
    }

    /// Start background retry processor (requires the service to be wrapped in Arc)
    pub fn start_retry_processor(service: Arc<NotificationService>) {
        let service_clone = service.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute

            loop {
                interval.tick().await;

                if let Err(e) = service_clone.process_retries().await {
                    error!("Error processing notification retries: {}", e);
                }
            }
        });

        info!("Started notification retry processor");
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}
