use crate::execution_simulator::ExecutionSimulator;
use crate::models::{
    ExecuteOrderRequest, OrderSide as LocalOrderSide, OrderType as LocalOrderType,
};
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType, OrderSide, OrderType};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct MarketDataEventConsumer {
    execution_simulator: Arc<ExecutionSimulator>,
    redis_url: String,
}

impl MarketDataEventConsumer {
    pub fn new(execution_simulator: Arc<ExecutionSimulator>, redis_url: String) -> Self {
        Self {
            execution_simulator,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("Market Data Service event consumer starting");

        let simulator = Arc::clone(&self.execution_simulator);
        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let sim = Arc::clone(&simulator);
                let event_clone = event.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_event_async(sim, event_clone).await {
                        error!(error = %e, "Failed to handle event");
                    }
                });
                Ok(())
            })
            .await
    }

    async fn handle_event_async(
        simulator: Arc<ExecutionSimulator>,
        event: OrderEventPayload,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderValidated => {
                let order_id = Uuid::parse_str(&event.order_id)?;
                let user_id = Uuid::parse_str(&event.user_id)?;

                let order_side = match event.order_side {
                    OrderSide::Buy => LocalOrderSide::Buy,
                    OrderSide::Sell => LocalOrderSide::Sell,
                };

                let order_type = match event.order_type {
                    OrderType::Market => LocalOrderType::Market,
                    OrderType::Limit { price } => LocalOrderType::Limit { price },
                };

                let execute_request = ExecuteOrderRequest {
                    order_id,
                    user_id,
                    symbol: event.symbol.clone(),
                    order_type,
                    side: order_side,
                    quantity: event.quantity as u64,
                };

                match simulator.execute_order(execute_request).await {
                    Ok(response) => {
                        info!(
                            order_id = %event.order_id,
                            symbol = %event.symbol,
                            quantity = event.quantity,
                            filled_quantity = response.filled_quantity,
                            average_price = ?response.average_price,
                            status = ?response.status,
                            "Order execution completed"
                        );
                    }
                    Err(e) => {
                        error!(
                            order_id = %event.order_id,
                            error = %e,
                            "Order execution failed"
                        );
                    }
                }

                metrics::counter!("market_data_events_processed_total", "event_type" => "validated")
                    .increment(1);
            }
            OrderEventType::OrderCancelled => {
                warn!(order_id = %event.order_id, "Order cancelled");

                metrics::counter!("market_data_events_processed_total", "event_type" => "cancelled")
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
