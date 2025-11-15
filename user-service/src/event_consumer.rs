use crate::service::UserService;
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType, OrderSide};
use std::sync::Arc;
use tracing::{info, warn};

pub struct UserEventConsumer {
    #[allow(dead_code)]
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

        subscriber
            .subscribe(move |event: OrderEventPayload| self.handle_event(event))
            .await
    }

    fn handle_event(
        &self,
        event: OrderEventPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderExecuted => {
                info!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    symbol = %event.symbol,
                    quantity = event.quantity,
                    "Order executed, should update holdings"
                );

                metrics::counter!("user_events_processed_total", "event_type" => "executed")
                    .increment(1);
            }
            OrderEventType::OrderFailed => {
                warn!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    "Order failed, should issue refund"
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
