use crate::service::MarketDataService;
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType};
use std::sync::Arc;
use tracing::{info, warn};

pub struct MarketDataEventConsumer {
    #[allow(dead_code)]
    market_data_service: Arc<MarketDataService>,
    redis_url: String,
}

impl MarketDataEventConsumer {
    pub fn new(market_data_service: Arc<MarketDataService>, redis_url: String) -> Self {
        Self {
            market_data_service,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("Market Data Service event consumer starting");

        subscriber
            .subscribe(move |event: OrderEventPayload| self.handle_event(event))
            .await
    }

    fn handle_event(
        &self,
        event: OrderEventPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderValidated => {
                info!(
                    order_id = %event.order_id,
                    symbol = %event.symbol,
                    quantity = event.quantity,
                    "Order validated, executing trade"
                );

                metrics::counter!("market_data_events_processed_total", "event_type" => "validated").increment(1);
            }
            OrderEventType::OrderCancelled => {
                warn!(
                    order_id = %event.order_id,
                    "Order cancelled"
                );

                metrics::counter!("market_data_events_processed_total", "event_type" => "cancelled").increment(1);
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
