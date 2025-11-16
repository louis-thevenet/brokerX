use crate::event_publisher::EventPublisher;
use crate::models::OrderStatus;
use crate::service::OrderService;
use crate::trade_validation::{TradeValidator, ValidationConfig};
use brokerx_events::{EventSubscriber, OrderEventPayload, OrderEventType};
use reqwest::Client;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct OrderEventConsumer {
    order_service: Arc<OrderService>,
    validator: Arc<TradeValidator>,
    event_publisher: Arc<EventPublisher>,
    redis_url: String,
}

impl OrderEventConsumer {
    pub fn new(
        order_service: Arc<OrderService>,
        redis_url: String,
        event_store_url: String,
        user_service_url: String,
        market_data_service_url: String,
    ) -> Self {
        let http_client = Arc::new(Client::new());
        let validator = Arc::new(TradeValidator::new(
            http_client.clone(),
            user_service_url,
            market_data_service_url,
            ValidationConfig::default(),
        ));
        let event_publisher = Arc::new(EventPublisher::new(event_store_url));

        Self {
            order_service,
            validator,
            event_publisher,
            redis_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("Order Service event consumer starting - listening for ORDER_CREATED");

        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let order_service = self.order_service.clone();
                let validator = self.validator.clone();
                let event_publisher = self.event_publisher.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        Self::handle_event_async(event, order_service, validator, event_publisher)
                            .await
                    {
                        error!("Error handling event: {}", e);
                    }
                });

                Ok(())
            })
            .await
    }

    async fn handle_event_async(
        event: OrderEventPayload,
        order_service: Arc<OrderService>,
        validator: Arc<TradeValidator>,
        event_publisher: Arc<EventPublisher>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event.event_type {
            OrderEventType::OrderCreated => {
                info!(
                    order_id = %event.order_id,
                    user_id = %event.user_id,
                    symbol = %event.symbol,
                    quantity = event.quantity,
                    order_side = ?event.order_side,
                    "Validating newly created order"
                );

                // Convert OrderEventPayload types to trade_validation types
                use crate::models::{OrderSide as ModelOrderSide, OrderType as ModelOrderType};

                let model_side = match event.order_side {
                    brokerx_events::OrderSide::Buy => ModelOrderSide::Buy,
                    brokerx_events::OrderSide::Sell => ModelOrderSide::Sell,
                };

                let model_type = match &event.order_type {
                    brokerx_events::OrderType::Market => ModelOrderType::Market,
                    brokerx_events::OrderType::Limit { price } => {
                        ModelOrderType::Limit { price: *price }
                    }
                };

                // Validate the trade
                let validation_result = validator
                    .validate_trade(
                        event.user_id.parse()?,
                        &event.symbol,
                        event.quantity.abs() as u64, // Convert i64 to u64
                        &model_side,
                        &model_type,
                    )
                    .await;

                if validation_result.valid {
                    info!(
                        order_id = %event.order_id,
                        "Order validation passed - publishing ORDER_VALIDATED"
                    );

                    // Publish ORDER_VALIDATED event
                    use crate::event_publisher::PublishEventRequest;
                    let validated_request = PublishEventRequest {
                        order_id: event.order_id.parse()?,
                        user_id: event.user_id.parse()?,
                        event_type: "ORDER_VALIDATED".to_string(),
                        symbol: event.symbol.clone(),
                        quantity: event.quantity.abs() as u64,
                        order_side: event.order_side.clone(),
                        order_type: event.order_type.clone(),
                        filled_quantity: None,
                        average_price: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    event_publisher.publish_event(validated_request).await?;
                    metrics::counter!("order_validation_total", "result" => "passed").increment(1);
                } else {
                    warn!(
                        order_id = %event.order_id,
                        errors = ?validation_result.errors,
                        "Order validation failed - publishing ORDER_REJECTED"
                    );

                    // Publish ORDER_REJECTED event
                    use crate::event_publisher::PublishEventRequest;
                    let failed_request = PublishEventRequest {
                        order_id: event.order_id.parse()?,
                        user_id: event.user_id.parse()?,
                        event_type: "ORDER_REJECTED".to_string(),
                        symbol: event.symbol.clone(),
                        quantity: event.quantity.abs() as u64,
                        order_side: event.order_side.clone(),
                        order_type: event.order_type.clone(),
                        filled_quantity: None,
                        average_price: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    event_publisher.publish_event(failed_request).await?;
                    metrics::counter!("order_validation_total", "result" => "rejected")
                        .increment(1);
                }
            }
            OrderEventType::OrderValidated => {
                info!(
                    order_id = %event.order_id,
                    "Order validated event received - updating status to Pending (awaiting execution)"
                );

                // Update order status to Pending (still waiting for execution)
                let order_id = event.order_id.parse()?;
                order_service
                    .update_order_status(&order_id, OrderStatus::Pending)
                    .await
                    .map_err(|e| format!("Failed to update order status: {}", e))?;

                metrics::counter!("order_events_processed_total", "event_type" => "validated")
                    .increment(1);
            }
            OrderEventType::OrderRejected => {
                warn!(
                    order_id = %event.order_id,
                    "Order rejected event received - updating status to Rejected"
                );

                // Update order status to Rejected
                let order_id = event.order_id.parse()?;
                let rejected_status = OrderStatus::Rejected {
                    date: chrono::Utc::now(),
                    reason: Some("Order validation failed".to_string()),
                };
                order_service
                    .update_order_status(&order_id, rejected_status)
                    .await
                    .map_err(|e| format!("Failed to update order status: {}", e))?;

                metrics::counter!("order_events_processed_total", "event_type" => "rejected")
                    .increment(1);
            }
            OrderEventType::OrderExecuted => {
                info!(
                    order_id = %event.order_id,
                    filled_quantity = ?event.filled_quantity,
                    average_price = ?event.average_price,
                    "Order executed event received - updating status to Filled"
                );

                // Update order status to Filled
                let order_id = event.order_id.parse()?;
                let filled_status = OrderStatus::Filled {
                    date: chrono::Utc::now(),
                };
                order_service
                    .update_order_status(&order_id, filled_status)
                    .await
                    .map_err(|e| format!("Failed to update order status: {}", e))?;

                metrics::counter!("order_events_processed_total", "event_type" => "executed")
                    .increment(1);
            }
            OrderEventType::OrderCompleted => {
                info!(
                    order_id = %event.order_id,
                    filled_quantity = ?event.filled_quantity,
                    average_price = ?event.average_price,
                    "Order completed event received - updating status to Filled"
                );

                // Update order status to Filled
                let order_id = event.order_id.parse()?;
                let filled_status = OrderStatus::Filled {
                    date: chrono::Utc::now(),
                };
                order_service
                    .update_order_status(&order_id, filled_status)
                    .await
                    .map_err(|e| format!("Failed to update order status: {}", e))?;

                metrics::counter!("order_events_processed_total", "event_type" => "completed")
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
