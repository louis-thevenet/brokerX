use crate::event_publisher::{EventPublisher, PublishEventRequest};
use crate::models::UserId;
use crate::service::UserService;
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType, OrderSide};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct UserEventConsumer {
    user_service: Arc<UserService>,
    event_publisher: Arc<EventPublisher>,
    redis_url: String,
}

impl UserEventConsumer {
    pub fn new(
        user_service: Arc<UserService>,
        event_publisher: Arc<EventPublisher>,
        redis_url: String,
    ) -> Self {
        Self {
            user_service,
            event_publisher,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("User Service event consumer starting");

        let user_service = Arc::clone(&self.user_service);
        let event_publisher = Arc::clone(&self.event_publisher);
        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let service = Arc::clone(&user_service);
                let publisher = Arc::clone(&event_publisher);
                let event_clone = event.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_event_async(service, publisher, event_clone).await
                    {
                        error!(error = %e, "Failed to handle event");
                    }
                });
                Ok(())
            })
            .await
    }

    async fn handle_event_async(
        user_service: Arc<UserService>,
        event_publisher: Arc<EventPublisher>,
        event: OrderEventPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderExecuted | OrderEventType::OrderPartiallyFilled => {
                let user_id = UserId::from_str(&event.user_id)?;
                let quantity = event.filled_quantity.unwrap_or(event.quantity);
                let price = event.average_price.ok_or("Missing average_price")?;

                let event_type_name = match event.event_type {
                    OrderEventType::OrderExecuted => "executed",
                    OrderEventType::OrderPartiallyFilled => "partially_filled",
                    _ => "unknown",
                };

                match event.order_side {
                    OrderSide::Buy => {
                        user_service
                            .update_user_holding(&user_id, &event.symbol, quantity, price)
                            .await?;

                        info!(
                            order_id = %event.order_id,
                            user_id = %event.user_id,
                            symbol = %event.symbol,
                            quantity = quantity,
                            price = price,
                            "Holdings updated after buy order {} (filled {} shares)",
                            event_type_name,
                            quantity
                        );
                    }
                    OrderSide::Sell => {
                        user_service
                            .update_user_holding(&user_id, &event.symbol, -quantity, price)
                            .await?;

                        info!(
                            order_id = %event.order_id,
                            user_id = %event.user_id,
                            symbol = %event.symbol,
                            quantity = quantity,
                            price = price,
                            "Holdings updated after sell order {} (filled {} shares)",
                            event_type_name,
                            quantity
                        );
                    }
                }

                // only publish ORDER_COMPLETED for fully filled orders, not partial fills
                if matches!(event.event_type, OrderEventType::OrderExecuted) {
                    let completed_event = PublishEventRequest {
                        order_id: uuid::Uuid::parse_str(&event.order_id)?,
                        user_id: uuid::Uuid::parse_str(&event.user_id)?,
                        event_type: "ORDER_COMPLETED".to_string(),
                        symbol: event.symbol.clone(),
                        quantity: event.quantity as u64,
                        order_side: event.order_side,
                        order_type: event.order_type,
                        filled_quantity: Some(quantity as u64),
                        average_price: Some(price),
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) = event_publisher.publish_event(completed_event).await {
                        error!(
                            order_id = %event.order_id,
                            error = %e,
                            "Failed to publish ORDER_COMPLETED event"
                        );
                    } else {
                        info!(
                            order_id = %event.order_id,
                            "ORDER_COMPLETED event published successfully"
                        );
                    }
                }

                metrics::counter!("user_events_processed_total", "event_type" => event_type_name)
                    .increment(1);
            }
            OrderEventType::OrderFailed => {
                warn!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    "Order failed, compensation may be needed"
                );

                metrics::counter!("user_events_processed_total", "event_type" => "failed")
                    .increment(1);
            }
            OrderEventType::RefundIssued => {
                info!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    "Refund issued"
                );

                metrics::counter!("user_events_processed_total", "event_type" => "refund")
                    .increment(1);
            }
            OrderEventType::HoldingsReverted => {
                info!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    "Holdings reverted"
                );

                metrics::counter!("user_events_processed_total", "event_type" => "revert")
                    .increment(1);
            }
            _ => {
                info!(
                    order_id = %event.order_id,
                    event_type = ?event.event_type,
                    "Ignoring event type"
                );
            }
        }

        Ok(())
    }
}
