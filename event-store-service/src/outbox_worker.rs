use crate::redis_client::RedisClient;
use crate::repository::EventStoreRepository;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Outbox Worker - Background task that polls outbox table and publishes to Redis
pub struct OutboxWorker {
    repository: EventStoreRepository,
    redis_client: Arc<RedisClient>,
    poll_interval: Duration,
    batch_size: i64,
}

impl OutboxWorker {
    /// Create a new Outbox Worker
    #[must_use]
    pub fn new(repository: EventStoreRepository, redis_client: Arc<RedisClient>) -> Self {
        Self {
            repository,
            redis_client,
            poll_interval: Duration::from_millis(100), // Poll every 100ms
            batch_size: 100,
        }
    }

    /// Start the worker (runs indefinitely)
    pub async fn start(self) {
        info!(
            "Starting Outbox Worker (polling every {:?}, batch size: {})",
            self.poll_interval, self.batch_size
        );

        let mut interval_timer = interval(self.poll_interval);
        let mut consecutive_errors = 0;
        let max_consecutive_errors = 10;

        loop {
            interval_timer.tick().await;

            match self.process_outbox_batch().await {
                Ok(processed) => {
                    if processed > 0 {
                        debug!("Processed {} outbox messages", processed);
                        metrics::counter!("outbox_messages_processed_total")
                            .increment(processed as u64);
                    }
                    consecutive_errors = 0; // Reset error counter on success
                }
                Err(e) => {
                    consecutive_errors += 1;
                    error!(
                        "Error processing outbox batch (attempt {}/{}): {}",
                        consecutive_errors, max_consecutive_errors, e
                    );
                    metrics::counter!("outbox_processing_errors_total").increment(1);

                    if consecutive_errors >= max_consecutive_errors {
                        error!("Too many consecutive errors, backing off for 5s...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        consecutive_errors = 0;
                    }
                }
            }
        }
    }

    /// Process a batch of unprocessed outbox messages
    async fn process_outbox_batch(&self) -> color_eyre::Result<usize> {
        // 1. Fetch unprocessed messages from outbox table
        let messages = self
            .repository
            .get_unprocessed_outbox(self.batch_size)
            .await?;

        if messages.is_empty() {
            return Ok(0);
        }

        debug!(
            "Found {} unprocessed outbox messages to publish",
            messages.len()
        );

        let mut processed_count = 0;

        for message in messages {
            let start_time = std::time::Instant::now();

            match self.process_single_message(&message).await {
                Ok(()) => {
                    // Mark message as processed
                    if let Err(e) = self
                        .repository
                        .mark_outbox_processed(message.outbox_id)
                        .await
                    {
                        error!(
                            "Failed to mark outbox message {} as processed: {}",
                            message.outbox_id, e
                        );
                    } else {
                        processed_count += 1;
                        let duration = start_time.elapsed();
                        metrics::histogram!("outbox_message_processing_duration_seconds")
                            .record(duration.as_secs_f64());

                        debug!(
                            "Published outbox message {} (event_type: {}) in {:?}",
                            message.outbox_id, message.event_type, duration
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to process outbox message {} (attempt {}): {}",
                        message.outbox_id,
                        message.processing_attempts + 1,
                        e
                    );

                    // Mark as failed with error message
                    if let Err(mark_err) = self
                        .repository
                        .mark_outbox_failed(message.outbox_id, &e.to_string())
                        .await
                    {
                        error!(
                            "Failed to mark outbox message {} as failed: {}",
                            message.outbox_id, mark_err
                        );
                    }

                    // Exponential backoff after 5 failed attempts
                    if message.processing_attempts >= 5 {
                        let backoff_seconds = 2_u64.pow(message.processing_attempts.min(10) as u32);
                        warn!(
                            "Message {} failed {} times, applying backoff of {}s",
                            message.outbox_id, message.processing_attempts, backoff_seconds
                        );
                        tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
                    }

                    metrics::counter!("outbox_messages_failed_total").increment(1);
                }
            }
        }

        if processed_count > 0 {
            info!("Published {} messages to Redis", processed_count);
        }

        Ok(processed_count)
    }

    /// Process a single outbox message
    async fn process_single_message(
        &self,
        message: &crate::models::OutboxMessage,
    ) -> color_eyre::Result<()> {
        debug!(
            "Publishing outbox message {} to Redis (event_type: {})",
            message.outbox_id, message.event_type
        );

        // Publish to Redis Pub/Sub on "order_events" channel
        self.redis_client
            .publish("order_events", &message.payload)
            .await?;

        info!(
            "Published event {} to Redis successfully",
            message.event_type
        );

        Ok(())
    }
}
