use futures_util::StreamExt;
use redis::Client;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use tracing::{debug, error, info, warn};

/// Event Subscriber - Base class for listening to Redis Pub/Sub events
pub struct EventSubscriber {
    redis_url: String,
    channel: String,
}

impl EventSubscriber {
    /// Create a new `EventSubscriber`
    #[must_use]
    pub fn new(redis_url: &str, channel: &str) -> Self {
        Self {
            redis_url: redis_url.to_string(),
            channel: channel.to_string(),
        }
    }

    /// Start listening to events and handle them with the provided handler
    ///
    /// # Arguments
    /// * `handler` - A closure that processes each event. Should return Result<(), Box<dyn std::error::Error>>
    pub async fn subscribe<T, F>(&self, mut handler: F) -> color_eyre::Result<()>
    where
        T: DeserializeOwned + Send + Debug + 'static,
        F: FnMut(T) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
    {
        info!(
            "EventSubscriber connecting to Redis at {} on channel '{}'",
            self.redis_url, self.channel
        );

        // Connect to Redis and create PubSub
        let client = Client::open(self.redis_url.as_str())?;
        let connection = client.get_async_connection().await?;

        let mut pubsub = connection.into_pubsub();
        pubsub.subscribe(&self.channel).await?;

        info!("EventSubscriber listening on channel '{}'", self.channel);

        // Infinite loop to receive messages
        loop {
            if let Some(msg) = pubsub.on_message().next().await {
                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to get payload: {}", e);
                        metrics::counter!("event_subscriber_payload_errors_total", "channel" => self.channel.clone())
                            .increment(1);
                        continue;
                    }
                };

                debug!(
                    "Received message on channel '{}': {}",
                    self.channel,
                    payload.chars().take(100).collect::<String>()
                );

                // Deserialize event
                match serde_json::from_str::<T>(&payload) {
                    Ok(event) => {
                        debug!("Deserialized event: {:?}", event);

                        // Handle event
                        if let Err(e) = handler(event) {
                            error!("Error handling event: {}", e);
                            metrics::counter!("event_handler_errors_total", "channel" => self.channel.clone())
                                .increment(1);
                        } else {
                            metrics::counter!("events_processed_total", "channel" => self.channel.clone())
                                .increment(1);
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize event: {}", e);
                        warn!("Payload was: {}", payload);
                        metrics::counter!("event_deserialization_errors_total", "channel" => self.channel.clone())
                            .increment(1);
                    }
                }
            } else {
                warn!("No message received, connection may be closed");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }
}
