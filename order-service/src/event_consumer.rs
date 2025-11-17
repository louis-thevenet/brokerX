use crate::event_publisher::EventPublisher;
use crate::event_publisher::PublishEventRequest;
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
    event_store_url: String,
}

impl OrderEventConsumer {
    #[must_use]
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
        let event_publisher = Arc::new(EventPublisher::new(event_store_url.clone()));

        Self {
            order_service,
            validator,
            event_publisher,
            redis_url,
            event_store_url,
        }
    }

    pub async fn start(self) -> color_eyre::Result<()> {
        let subscriber = EventSubscriber::new(&self.redis_url, "order_events");

        info!("Order Service event consumer starting - listening for ORDER_CREATED");

        let event_store_url = self.event_store_url.clone();
        let redis_url = self.redis_url.clone();

        subscriber
            .subscribe(move |event: OrderEventPayload| {
                let order_service = self.order_service.clone();
                let validator = self.validator.clone();
                let event_publisher = self.event_publisher.clone();
                let event_store_url = event_store_url.clone();
                let redis_url = redis_url.clone();

                tokio::spawn(async move {
                    if let Err(e) = Self::handle_event_async(
                        event,
                        order_service,
                        validator,
                        event_publisher,
                        event_store_url,
                        redis_url,
                    )
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
        event_store_url: String,
        redis_url: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // IDEMPOTENCY CHECK
        let event_id =
            uuid::Uuid::parse_str(&event.event_id).map_err(|e| format!("Invalid event_id: {e}"))?;

        let is_processed = order_service
            .is_event_processed(&event_id)
            .await
            .map_err(|e| format!("Failed to check event idempotency: {e}"))?;

        if is_processed {
            info!(
                event_id = %event.event_id,
                order_id = %event.order_id,
                event_type = ?event.event_type,
                "Event already processed (idempotency), skipping"
            );
            metrics::counter!("order_events_deduplicated_total", "event_type" => format!("{:?}", event.event_type))
                .increment(1);
            return Ok(());
        }

        // Process event and mark as processed
        let result = Self::process_event(
            &event,
            order_service.clone(),
            validator,
            event_publisher,
            &event_store_url,
            &redis_url,
        )
        .await;

        // Mark event as processed (success or failure) to prevent reprocessing
        let processing_result = if result.is_ok() { "success" } else { "failure" };
        let order_id =
            uuid::Uuid::parse_str(&event.order_id).map_err(|e| format!("Invalid order_id: {e}"))?;
        let event_payload =
            serde_json::to_value(&event).map_err(|e| format!("Failed to serialize event: {e}"))?;

        order_service
            .mark_event_processed(
                event_id,
                order_id,
                &format!("{:?}", event.event_type),
                event_payload,
                processing_result,
            )
            .await
            .map_err(|e| format!("Failed to mark event as processed: {e}"))?;

        result
    }

    async fn process_event(
        event: &OrderEventPayload,
        order_service: Arc<OrderService>,
        validator: Arc<TradeValidator>,
        event_publisher: Arc<EventPublisher>,
        event_store_url: &str,
        redis_url: &str,
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
                        event.quantity.unsigned_abs(), // Convert i64 to u64
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
                        quantity: event.quantity.unsigned_abs(),
                        order_side: event.order_side.clone(),
                        order_type: event.order_type.clone(),
                        filled_quantity: None,
                        average_price: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    event_publisher.publish_event(validated_request).await?;
                    metrics::counter!("order_validation_total", "result" => "passed").increment(1);

                    // Update saga state to VALIDATED
                    Self::update_saga_state(event_store_url, &event.order_id, "VALIDATED").await?;
                } else {
                    warn!(
                        order_id = %event.order_id,
                        errors = ?validation_result.errors,
                        "Order validation failed - publishing ORDER_REJECTED"
                    );

                    // Publish ORDER_REJECTED event
                    let failed_request = PublishEventRequest {
                        order_id: event.order_id.parse()?,
                        user_id: event.user_id.parse()?,
                        event_type: "ORDER_REJECTED".to_string(),
                        symbol: event.symbol.clone(),
                        quantity: event.quantity.unsigned_abs(),
                        order_side: event.order_side.clone(),
                        order_type: event.order_type.clone(),
                        filled_quantity: None,
                        average_price: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    event_publisher.publish_event(failed_request).await?;
                    metrics::counter!("order_validation_total", "result" => "rejected")
                        .increment(1);

                    // Update saga state to FAILED for rejected orders
                    Self::update_saga_state(event_store_url, &event.order_id, "FAILED").await?;
                }
            }
            OrderEventType::OrderValidated => {
                let order_id = event.order_id.parse()?;

                // Check if this is a retry (order already has PartiallyFilled status)
                // If so, don't reset to Pending : keep the PartiallyFilled status
                let (_id, current_order) = order_service
                    .get_order(&order_id)
                    .await
                    .map_err(|e| format!("Failed to get order: {e}"))?
                    .ok_or("Order not found")?;

                if let OrderStatus::PartiallyFilled { .. } = current_order.status {
                    info!(
                        order_id = %event.order_id,
                        "Order validated event received for partially filled order - keeping PartiallyFilled status (retry)"
                    );
                    // keep PartiallyFilled
                } else {
                    info!(
                        order_id = %event.order_id,
                        "Order validated event received - updating status to Pending (awaiting execution)"
                    );
                    // set to Pending
                    order_service
                        .update_order_status(&order_id, OrderStatus::Pending)
                        .await
                        .map_err(|e| format!("Failed to update order status: {e}"))?;
                }

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
                    .map_err(|e| format!("Failed to update order status: {e}"))?;

                metrics::counter!("order_events_processed_total", "event_type" => "rejected")
                    .increment(1);

                // Publish notification event for rejection
                if let Err(e) = Self::publish_notification_event(event, redis_url).await {
                    error!(
                        "Failed to publish notification event for order {}: {}",
                        event.order_id, e
                    );
                }
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
                    .map_err(|e| format!("Failed to update order status: {e}"))?;

                metrics::counter!("order_events_processed_total", "event_type" => "executed")
                    .increment(1);

                // Publish notification event for immediate execution
                if let Err(e) = Self::publish_notification_event(event, redis_url).await {
                    error!(
                        "Failed to publish notification event for order {}: {}",
                        event.order_id, e
                    );
                }

                // Update saga state to EXECUTED
                Self::update_saga_state(event_store_url, &event.order_id, "EXECUTED").await?;
            }
            OrderEventType::OrderPartiallyFilled => {
                let new_filled_qty = event.filled_quantity.unwrap_or(0).unsigned_abs();
                let order_id = event.order_id.parse()?;

                info!(
                    order_id = %event.order_id,
                    new_filled = new_filled_qty,
                    "Order partially filled - using atomic cumulative fill update"
                );

                let cumulative_filled = order_service
                    .atomic_increment_filled_quantity(&order_id, new_filled_qty)
                    .await
                    .map_err(|e| {
                        error!(
                            order_id = %event.order_id,
                            error = %e,
                            "Failed to atomically increment filled quantity"
                        );
                        format!("Atomic update failed: {e}")
                    })?;

                // Get updated order to check status and remaining quantity
                let (_id, current_order) = order_service
                    .get_order(&order_id)
                    .await
                    .map_err(|e| format!("Failed to get order: {e}"))?
                    .ok_or("Order not found")?;

                let remaining_qty = current_order.quantity.saturating_sub(cumulative_filled);

                info!(
                    order_id = %event.order_id,
                    new_filled = new_filled_qty,
                    cumulative_filled = cumulative_filled,
                    total_quantity = current_order.quantity,
                    remaining_quantity = remaining_qty,
                    status = ?current_order.status,
                    "Atomic partial fill update complete"
                );

                if matches!(current_order.status, OrderStatus::Filled { .. }) {
                    // Order completed - send "filled" notification
                    info!(
                        order_id = %event.order_id,
                        cumulative_filled = cumulative_filled,
                        "Order completed through partial fills - sending FILLED notification"
                    );

                    // Create a NEW completion event
                    let completion_event = OrderEventPayload {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        order_id: event.order_id.clone(),
                        user_id: event.user_id.clone(),
                        event_type: OrderEventType::OrderCompleted,
                        symbol: event.symbol.clone(),
                        quantity: current_order.quantity as i64,
                        order_side: event.order_side.clone(),
                        order_type: event.order_type.clone(),
                        price: event.price,
                        filled_quantity: Some(cumulative_filled as i64),
                        average_price: event.average_price,
                        correlation_id: event.correlation_id.clone(),
                        causation_id: Some(event.event_id.clone()),
                        metadata: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) =
                        Self::publish_notification_event(&completion_event, redis_url).await
                    {
                        error!(
                            "Failed to publish completion notification for order {}: {}",
                            event.order_id, e
                        );
                    }

                    metrics::counter!("order_events_processed_total", "event_type" => "completed_via_partial_fill")
                        .increment(1);
                } else {
                    // Order still partial - send "partially_filled" notification
                    info!(
                        order_id = %event.order_id,
                        cumulative_filled = cumulative_filled,
                        remaining_quantity = remaining_qty,
                        "📊 Order still partial - sending PARTIAL notification"
                    );

                    // Update the event with cumulative filled quantity
                    let mut partial_event = event.clone();
                    partial_event.filled_quantity = Some(cumulative_filled as i64);

                    if let Err(e) =
                        Self::publish_notification_event(&partial_event, redis_url).await
                    {
                        error!(
                            "Failed to publish partial fill notification for order {}: {}",
                            event.order_id, e
                        );
                    }

                    // Re-publish ORDER_VALIDATED for remaining quantity
                    if remaining_qty > 0 {
                        let retry_event = crate::event_publisher::PublishEventRequest {
                            order_id: uuid::Uuid::parse_str(&event.order_id)?,
                            user_id: uuid::Uuid::parse_str(&event.user_id)?,
                            event_type: "ORDER_VALIDATED".to_string(),
                            symbol: event.symbol.clone(),
                            quantity: remaining_qty,
                            order_side: event.order_side.clone(),
                            order_type: event.order_type.clone(),
                            filled_quantity: Some(cumulative_filled),
                            average_price: event.average_price,
                            occurred_at: chrono::Utc::now().to_rfc3339(),
                        };

                        if let Err(e) = event_publisher.publish_event(retry_event).await {
                            error!(
                                order_id = %event.order_id,
                                error = %e,
                                "Failed to publish retry ORDER_VALIDATED event"
                            );
                        }
                    }

                    metrics::counter!("order_events_processed_total", "event_type" => "partially_filled")
                        .increment(1);
                }
            }
            OrderEventType::OrderFailed => {
                warn!(
                    order_id = %event.order_id,
                    "Order execution failed - updating status to Rejected"
                );

                let order_id = event.order_id.parse()?;
                let rejected_status = OrderStatus::Rejected {
                    date: chrono::Utc::now(),
                    reason: Some("Order execution failed".to_string()),
                };
                order_service
                    .update_order_status(&order_id, rejected_status)
                    .await
                    .map_err(|e| format!("Failed to update order status: {e}"))?;

                metrics::counter!("order_events_processed_total", "event_type" => "failed")
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
                    .map_err(|e| format!("Failed to update order status: {e}"))?;

                metrics::counter!("order_events_processed_total", "event_type" => "completed")
                    .increment(1);

                // NOTE: Notification already sent by OrderPartiallyFilled handler (when it detects completion)
                // OR by OrderExecuted handler (for immediate fills)
                // No need to send duplicate here

                // Update saga state to COMPLETED
                Self::update_saga_state(event_store_url, &event.order_id, "COMPLETED").await?;
            }
            OrderEventType::OrderCancelled => {
                info!(
                    order_id = %event.order_id,
                    "Order cancelled event received - updating status to Cancelled"
                );

                // Update order status to Cancelled
                let order_id = event.order_id.parse()?;
                let cancelled_status = OrderStatus::Cancelled {
                    date: chrono::Utc::now(),
                };
                order_service
                    .update_order_status(&order_id, cancelled_status)
                    .await
                    .map_err(|e| format!("Failed to update order status: {e}"))?;

                metrics::counter!("order_events_processed_total", "event_type" => "cancelled")
                    .increment(1);

                // Update saga state to FAILED for cancelled orders
                Self::update_saga_state(event_store_url, &event.order_id, "FAILED").await?;
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

    // Helper function to publish notification events
    async fn publish_notification_event(
        original_event: &OrderEventPayload,
        redis_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Publish the original event directly to notification_events channel
        // This preserves the original event type (OrderExecuted, OrderCompleted, etc.)
        // so the notification service can handle it appropriately
        let redis_client = redis::Client::open(redis_url)?;
        let mut redis_conn = redis_client.get_async_connection().await?;

        let notification_payload = serde_json::to_string(original_event)?;
        let _: () = redis::cmd("PUBLISH")
            .arg("notification_events")
            .arg(&notification_payload)
            .query_async(&mut redis_conn)
            .await?;

        info!(
            "Published notification event for order {} (trigger: {:?})",
            original_event.order_id, original_event.event_type
        );

        Ok(())
    }
    async fn update_saga_state(
        event_store_url: &str,
        order_id: &str,
        step: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::new();
        let url = format!("{}/v1/saga/{}/step", event_store_url, order_id);

        let payload = serde_json::json!({
            "step": step
        });

        let response = client
            .put(&url)
            .json(&payload)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;

        if response.status().is_success() {
            info!(
                "Successfully updated saga state for order {} to {}",
                order_id, step
            );
            Ok(())
        } else {
            warn!(
                "Failed to update saga state for order {}: HTTP {}",
                order_id,
                response.status()
            );
            Ok(()) // Don't fail the entire event processing for saga state update failures
        }
    }
}
