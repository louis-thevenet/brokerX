use brokerx_events::{OrderSide, OrderType};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishEventRequest {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub event_type: String,
    pub symbol: String,
    pub quantity: u64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    pub filled_quantity: Option<u64>,
    pub average_price: Option<f64>,
    pub occurred_at: String, // ISO 8601 timestamp
}

pub struct EventPublisher {
    client: Client,
    event_store_url: String,
}

impl EventPublisher {
    #[must_use]
    pub fn new(event_store_url: String) -> Self {
        Self {
            client: Client::new(),
            event_store_url,
        }
    }

    pub async fn publish_event(
        &self,
        event: PublishEventRequest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/events", self.event_store_url);

        info!(
            order_id = %event.order_id,
            event_type = %event.event_type,
            "Publishing event to Event Store"
        );

        match self
            .client
            .post(&url)
            .json(&event)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    info!(
                        order_id = %event.order_id,
                        event_type = %event.event_type,
                        "Event published successfully"
                    );
                    metrics::counter!("events_published_total", "event_type" => event.event_type.clone()).increment(1);
                    Ok(())
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    error!(
                        order_id = %event.order_id,
                        status = %status,
                        body = %body,
                        "Failed to publish event"
                    );
                    metrics::counter!("events_publish_errors_total", "event_type" => event.event_type).increment(1);
                    Err(format!("Event Store returned {status}: {body}").into())
                }
            }
            Err(e) => {
                error!(
                    order_id = %event.order_id,
                    error = %e,
                    "Failed to send event to Event Store"
                );
                metrics::counter!("events_publish_errors_total", "event_type" => event.event_type)
                    .increment(1);
                Err(e.into())
            }
        }
    }
}
