use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType};
use std::sync::Arc;
use tracing::{info, warn};
use crate::service::OrderService;

pub struct OrderEventConsumer {
    #[allow(dead_code)]
    order_service: Arc<OrderService>,
    redis_url: String,
}

impl OrderEventConsumer {
    pub fn new(order_service: Arc<OrderService>, redis_url: String) -> Self {
        Self {
            order_service,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");
        
        info!("Order Service event consumer starting");

        subscriber.subscribe(move |event: OrderEventPayload| {
            self.handle_event(event)
        }).await
    }

    fn handle_event(&self, event: OrderEventPayload) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderValidated => {
                info!(
                    order_id = %event.order_id,
                    "Order validated event received"
                );
                metrics::counter!("order_events_processed_total", "event_type" => "validated").increment(1);
            }
            OrderEventType::OrderFailed => {
                warn!(
                    order_id = %event.order_id,
                    "Order failed event received"
                );
                metrics::counter!("order_events_processed_total", "event_type" => "failed").increment(1);
            }
            OrderEventType::OrderCompleted => {
                info!(
                    order_id = %event.order_id,
                    filled_quantity = ?event.filled_quantity,
                    average_price = ?event.average_price,
                    "Order completed event received"
                );
                metrics::counter!("order_events_processed_total", "event_type" => "completed").increment(1);
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
