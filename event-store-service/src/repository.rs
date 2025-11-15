use crate::models::{
    CorrelationId, OrderEvent, OrderEventType, OrderId, OrderSide, OrderType, OutboxMessage,
    PublishEventRequest, SagaState, SagaStep, UserId,
};
use chrono::Utc;
use sqlx::{PgPool, Row};
use tracing::{debug, info};
use uuid::Uuid;

/// Repository for event store operations
#[derive(Clone)]
pub struct EventStoreRepository {
    pool: PgPool,
}

impl EventStoreRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Save an event and add it to the outbox in a single transaction
    /// This implements the Transactional Outbox Pattern
    pub async fn save_event_with_outbox(
        &self,
        request: &PublishEventRequest,
    ) -> color_eyre::Result<OrderEvent> {
        let event_id = Uuid::new_v4();
        let correlation_id = request.correlation_id.unwrap_or_else(Uuid::new_v4);
        let now = Utc::now();

        // Start transaction
        let mut tx = self.pool.begin().await?;

        debug!(
            "Saving event {} for order {}",
            request.event_type, request.order_id
        );

        // Convert OrderType to JSONB for database
        let order_type_json = serde_json::to_value(&request.order_type)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize order_type: {e}")))?;

        let order_side_str = request.order_side.to_string();

        // 1. Insert into order_events
        let row = sqlx::query(
            r"
            INSERT INTO order_events (
                event_id, order_id, user_id, event_type,
                symbol, quantity, order_side, order_type, price,
                filled_quantity, average_price,
                occurred_at, correlation_id, causation_id, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING sequence_number
            ",
        )
        .bind(event_id)
        .bind(request.order_id)
        .bind(request.user_id)
        .bind(request.event_type.to_string())
        .bind(&request.symbol)
        .bind(request.quantity)
        .bind(order_side_str)
        .bind(order_type_json)
        .bind(request.price)
        .bind(request.filled_quantity.unwrap_or(0))
        .bind(request.average_price)
        .bind(now)
        .bind(correlation_id)
        .bind(request.causation_id)
        .bind(&request.metadata)
        .fetch_one(&mut *tx)
        .await?;

        let sequence_number: i64 = row.get("sequence_number");

        info!(
            "Event {} saved with sequence_number {}",
            event_id, sequence_number
        );

        // 2. Create payload for outbox
        let payload = serde_json::json!({
            "event_id": event_id,
            "order_id": request.order_id,
            "user_id": request.user_id,
            "event_type": request.event_type,
            "symbol": request.symbol,
            "quantity": request.quantity,
            "order_side": request.order_side,
            "order_type": request.order_type,
            "price": request.price,
            "filled_quantity": request.filled_quantity,
            "average_price": request.average_price,
            "correlation_id": correlation_id,
            "causation_id": request.causation_id,
            "metadata": request.metadata,
            "sequence_number": sequence_number,
        });

        // 3. Insert into outbox
        sqlx::query(
            r"
            INSERT INTO outbox (order_id, event_type, payload)
            VALUES ($1, $2, $3)
            ",
        )
        .bind(request.order_id)
        .bind(request.event_type.to_string())
        .bind(&payload)
        .execute(&mut *tx)
        .await?;

        // 4. Commit transaction
        tx.commit().await?;

        debug!("Event {} and outbox message committed", event_id);

        // Return the created event
        Ok(OrderEvent {
            event_id,
            order_id: request.order_id,
            user_id: request.user_id,
            event_type: request.event_type.clone(),
            symbol: request.symbol.clone(),
            quantity: request.quantity,
            order_side: request.order_side.clone(),
            order_type: request.order_type.clone(),
            price: request.price,
            filled_quantity: request.filled_quantity,
            average_price: request.average_price,
            occurred_at: now,
            sequence_number: Some(sequence_number),
            correlation_id,
            causation_id: request.causation_id,
            metadata: request.metadata.clone(),
        })
    }

    /// Get all events for a specific order
    pub async fn get_order_events(&self, order_id: OrderId) -> color_eyre::Result<Vec<OrderEvent>> {
        debug!("Fetching events for order {}", order_id);

        let rows = sqlx::query(
            r"
            SELECT 
                event_id, order_id, user_id, event_type,
                symbol, quantity, order_side, order_type, price,
                filled_quantity, average_price,
                occurred_at, sequence_number,
                correlation_id, causation_id, metadata
            FROM order_events
            WHERE order_id = $1
            ORDER BY sequence_number ASC
            ",
        )
        .bind(order_id)
        .fetch_all(&self.pool)
        .await?;

        let events: Vec<OrderEvent> = rows
            .iter()
            .map(|row| {
                let event_type_str: String = row.get("event_type");
                let event_type = serde_json::from_value(serde_json::json!(event_type_str))
                    .unwrap_or(OrderEventType::OrderCreated);

                let order_side_str: String = row.get("order_side");
                let order_side = if order_side_str == "buy" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                };

                let order_type_json: serde_json::Value = row.get("order_type");
                let order_type =
                    serde_json::from_value(order_type_json).unwrap_or(OrderType::Market);

                let price: Option<f64> = row.get("price");

                OrderEvent {
                    event_id: row.get("event_id"),
                    order_id: row.get("order_id"),
                    user_id: row.get("user_id"),
                    event_type,
                    symbol: row.get("symbol"),
                    quantity: row.get("quantity"),
                    order_side,
                    order_type,
                    price,
                    filled_quantity: Some(row.get::<i64, _>("filled_quantity")),
                    average_price: row.get("average_price"),
                    occurred_at: row.get("occurred_at"),
                    sequence_number: Some(row.get("sequence_number")),
                    correlation_id: row.get("correlation_id"),
                    causation_id: row.get("causation_id"),
                    metadata: row.get("metadata"),
                }
            })
            .collect();

        debug!("Found {} events for order {}", events.len(), order_id);

        Ok(events)
    }

    /// Get events by user
    pub async fn get_user_events(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> color_eyre::Result<Vec<OrderEvent>> {
        debug!(
            "Fetching events for user {} (limit={}, offset={})",
            user_id, limit, offset
        );

        let rows = sqlx::query(
            r"
            SELECT 
                event_id, order_id, user_id, event_type,
                symbol, quantity, order_side, order_type, price,
                filled_quantity, average_price,
                occurred_at, sequence_number,
                correlation_id, causation_id, metadata
            FROM order_events
            WHERE user_id = $1
            ORDER BY occurred_at DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let events: Vec<OrderEvent> = rows
            .iter()
            .map(|row| {
                let event_type_str: String = row.get("event_type");
                let event_type = serde_json::from_value(serde_json::json!(event_type_str))
                    .unwrap_or(OrderEventType::OrderCreated);

                let order_side_str: String = row.get("order_side");
                let order_side = if order_side_str == "buy" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                };

                let order_type_json: serde_json::Value = row.get("order_type");
                let order_type =
                    serde_json::from_value(order_type_json).unwrap_or(OrderType::Market);

                let price: Option<f64> = row.get("price");

                OrderEvent {
                    event_id: row.get("event_id"),
                    order_id: row.get("order_id"),
                    user_id: row.get("user_id"),
                    event_type,
                    symbol: row.get("symbol"),
                    quantity: row.get("quantity"),
                    order_side,
                    order_type,
                    price,
                    filled_quantity: Some(row.get::<i64, _>("filled_quantity")),
                    average_price: row.get("average_price"),
                    occurred_at: row.get("occurred_at"),
                    sequence_number: Some(row.get("sequence_number")),
                    correlation_id: row.get("correlation_id"),
                    causation_id: row.get("causation_id"),
                    metadata: row.get("metadata"),
                }
            })
            .collect();

        Ok(events)
    }

    // ========================================================================
    // Saga State Operations
    // ========================================================================

    /// Initialize saga state for a new order
    pub async fn init_saga_state(
        &self,
        order_id: OrderId,
        user_id: UserId,
        symbol: &str,
        quantity: i64,
        order_side: &OrderSide,
        order_type: &OrderType,
        price: Option<f64>,
        correlation_id: CorrelationId,
    ) -> color_eyre::Result<()> {
        debug!("Initializing saga state for order {}", order_id);

        let order_type_json = serde_json::to_value(order_type)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize order_type: {e}")))?;

        sqlx::query(
            r"
            INSERT INTO saga_state (
                order_id, user_id, current_step,
                symbol, quantity, order_side, order_type, price,
                correlation_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (order_id) DO NOTHING
            ",
        )
        .bind(order_id)
        .bind(user_id)
        .bind(SagaStep::Created.to_string())
        .bind(symbol)
        .bind(quantity)
        .bind(order_side.to_string())
        .bind(order_type_json)
        .bind(price)
        .bind(correlation_id)
        .execute(&self.pool)
        .await?;

        debug!("Saga state initialized for order {}", order_id);

        Ok(())
    }

    /// Update saga state
    pub async fn update_saga_state(
        &self,
        order_id: OrderId,
        new_step: SagaStep,
        filled_quantity: Option<i64>,
        average_price: Option<f64>,
        error_message: Option<String>,
        compensation_needed: bool,
    ) -> color_eyre::Result<()> {
        debug!(
            "Updating saga state for order {} to step {:?}",
            order_id, new_step
        );

        let completed_at = if new_step == SagaStep::Completed || new_step == SagaStep::Compensated {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r"
            UPDATE saga_state
            SET current_step = $2,
                filled_quantity = COALESCE($3, filled_quantity),
                average_price = COALESCE($4, average_price),
                error_message = $5,
                compensation_needed = $6,
                completed_at = $7
            WHERE order_id = $1
            ",
        )
        .bind(order_id)
        .bind(new_step.to_string())
        .bind(filled_quantity)
        .bind(average_price)
        .bind(error_message)
        .bind(compensation_needed)
        .bind(completed_at)
        .execute(&self.pool)
        .await?;

        debug!("Saga state updated for order {}", order_id);

        Ok(())
    }

    /// Get saga state for an order
    pub async fn get_saga_state(&self, order_id: OrderId) -> color_eyre::Result<Option<SagaState>> {
        debug!("Fetching saga state for order {}", order_id);

        let row = sqlx::query(
            r"
            SELECT 
                order_id, user_id, current_step,
                symbol, quantity, order_side, order_type, price,
                started_at, updated_at, completed_at,
                filled_quantity, average_price,
                error_message, compensation_needed,
                correlation_id, metadata
            FROM saga_state
            WHERE order_id = $1
            ",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let current_step_str: String = row.get("current_step");
            let current_step = serde_json::from_value(serde_json::json!(current_step_str))
                .unwrap_or(SagaStep::Created);

            let order_side_str: String = row.get("order_side");
            let order_side = if order_side_str == "buy" {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };

            let order_type_json: serde_json::Value = row.get("order_type");
            let order_type = serde_json::from_value(order_type_json).unwrap_or(OrderType::Market);

            let price: Option<f64> = row.get("price");

            Ok(Some(SagaState {
                order_id: row.get("order_id"),
                user_id: row.get("user_id"),
                current_step,
                symbol: row.get("symbol"),
                quantity: row.get("quantity"),
                order_side,
                order_type,
                price,
                started_at: row.get("started_at"),
                updated_at: row.get("updated_at"),
                completed_at: row.get("completed_at"),
                filled_quantity: row.get("filled_quantity"),
                average_price: row.get("average_price"),
                error_message: row.get("error_message"),
                compensation_needed: row.get("compensation_needed"),
                correlation_id: row.get("correlation_id"),
                metadata: row.get("metadata"),
            }))
        } else {
            Ok(None)
        }
    }

    // ========================================================================
    // Outbox Operations
    // ========================================================================

    /// Get unprocessed outbox messages (for the outbox worker)
    pub async fn get_unprocessed_outbox(
        &self,
        limit: i64,
    ) -> color_eyre::Result<Vec<OutboxMessage>> {
        let rows = sqlx::query(
            r"
            SELECT outbox_id, order_id, event_type, payload, 
                   created_at, processed_at, processing_attempts, last_error
            FROM outbox
            WHERE processed_at IS NULL
            ORDER BY created_at ASC
            LIMIT $1
            ",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let messages = rows
            .iter()
            .map(|row| {
                let event_type_str: String = row.get("event_type");
                let event_type = serde_json::from_value(serde_json::json!(event_type_str))
                    .unwrap_or(OrderEventType::OrderCreated);

                OutboxMessage {
                    outbox_id: row.get("outbox_id"),
                    order_id: row.get("order_id"),
                    event_type,
                    payload: row.get("payload"),
                    created_at: row.get("created_at"),
                    processed_at: row.get("processed_at"),
                    processing_attempts: row.get("processing_attempts"),
                    last_error: row.get("last_error"),
                }
            })
            .collect();

        Ok(messages)
    }

    /// Mark outbox message as processed
    pub async fn mark_outbox_processed(&self, outbox_id: Uuid) -> color_eyre::Result<()> {
        sqlx::query(
            r"
            UPDATE outbox
            SET processed_at = NOW()
            WHERE outbox_id = $1
            ",
        )
        .bind(outbox_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark outbox message as failed
    pub async fn mark_outbox_failed(&self, outbox_id: Uuid, error: &str) -> color_eyre::Result<()> {
        sqlx::query(
            r"
            UPDATE outbox
            SET processing_attempts = processing_attempts + 1,
                last_error = $2
            WHERE outbox_id = $1
            ",
        )
        .bind(outbox_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
