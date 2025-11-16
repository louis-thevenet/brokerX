use crate::models::UserId;
use crate::service::UserService;
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType, OrderSide};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct UserEventConsumer {
    user_service: Arc<UserService>,
    redis_url: String,
}

impl UserEventConsumer {
    pub fn new(user_service: Arc<UserService>, redis_url: String) -> Self {
        Self {
            user_service,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("User Service event consumer starting");

        let user_service = Arc::clone(&self.user_service);
        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let service = Arc::clone(&user_service);
                let event_clone = event.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_event_async(service, event_clone).await {
                        error!(error = %e, "Failed to handle event");
                    }
                });
                Ok(())
            })
            .await
    }

    async fn handle_event_async(
        user_service: Arc<UserService>,
        event: OrderEventPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderExecuted => {
                let user_id = UserId::from_str(&event.user_id)?;
                let quantity = event.filled_quantity.unwrap_or(event.quantity);
                let price = event.average_price.ok_or("Missing average_price")?;

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
                            "Holdings updated after buy order execution"
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
                            "Holdings updated after sell order execution"
                        );
                    }
                }

                metrics::counter!("user_events_processed_total", "event_type" => "executed")
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
