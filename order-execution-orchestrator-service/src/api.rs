use crate::{
    models::{CreateOrderRequest, OrderSide},
    AppState,
};
use axum::http::StatusCode;
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Create a new order (async, enqueue for processing)
/// Archestrates the full order lifecycle: validation, creation, execution, and updating user holdings.
#[utoipa::path(
    post,
    path = "/v1/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 202, description = "Order accepted for processing"),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid order request")
    ),
    security(("bearer_auth" = [])),
    tag = "orders"
)]
pub async fn create_order(
    State(state): State<AppState>,
    Json(order): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    // Extract user_id from the order request (set by API Gateway from authenticated user)
    let user_id = order.user_id;
    let queue = state.order_queue.as_ref();

    // Use send() for unbounded channel (does not block)
    if let Err(e) = queue.send((user_id, order)) {
        error!("Failed to enqueue order for user {}: {}", user_id, e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Increment queue length metric
    state.metrics.increment_queue();

    StatusCode::ACCEPTED
}

/// Processes orders concurrently for maximum throughput
pub async fn order_processor(
    mut rx: mpsc::UnboundedReceiver<(Uuid, CreateOrderRequest)>,
    state: Arc<AppState>,
) {
    info!("Order processor task started - processing orders concurrently with limits");
    let mut received_count = 0;

    while let Some((user_id, order_request)) = rx.recv().await {
        received_count += 1;
        if received_count % 500 == 0 {
            info!("Received {} orders so far...", received_count);
        }

        // Decrement queue length metric
        state.metrics.decrement_queue();

        // Acquire permit before spawning task (limits concurrent processing)
        let permit = state
            .concurrency_limiter
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        // Spawn a new task for each order to process them concurrently
        let state_clone = state.clone();
        tokio::spawn(async move {
            // Process the order asynchronously with full validation and execution
            if let Err(e) = process_single_order(user_id, order_request, &state_clone).await {
                error!("Failed to process order for user {}: {}", user_id, e);
            } else {
                // Record successful processing
                state_clone.metrics.record_order_processed();
            }
            // Permit is automatically released when dropped
            drop(permit);
        });
    }

    info!(
        "Order processor finished - channel closed after receiving {} orders",
        received_count
    );
}

/// Process a single order through the full lifecycle
async fn process_single_order(
    user_id: Uuid,
    order_request: CreateOrderRequest,
    state: &AppState,
) -> Result<(), String> {
    // PRE-TRADE VALIDATION
    info!("Running pre-trade validation for user {}", user_id);
    let validation_result = state
        .trade_validator
        .validate_trade(
            user_id,
            &order_request.symbol,
            order_request.quantity,
            &order_request.order_side,
            &order_request.order_type,
        )
        .await;

    if !validation_result.valid {
        let error_msg = validation_result
            .first_error()
            .map_or_else(|| "Validation failed".to_string(), ToString::to_string);

        error!(
            "Pre-trade validation failed for user {}: {}",
            user_id, error_msg
        );
        return Err(format!("Validation failed: {error_msg}"));
    }

    // Log any warnings
    for warning in &validation_result.warnings {
        warn!("Pre-trade warning for user {}: {}", user_id, warning);
    }

    info!("Pre-trade validation passed for user {}", user_id);

    // ORDER CREATION
    let order = match state
        .order_client
        .create_order(user_id, &order_request)
        .await
    {
        Ok(o) => o,
        Err(e) => {
            error!("Order creation failed for user {}: {}", user_id, e);
            return Err(format!("Order creation failed: {e}"));
        }
    };
    info!("Order {} created with status Pending", order.id);

    // ORDER EXECUTION
    let execution_request = crate::clients::ExecuteOrderRequest {
        order_id: order.id,
        symbol: order_request.symbol.clone(),
        side: order_request.order_side.clone(),
        order_type: order_request.order_type.clone(),
        quantity: order_request.quantity,
    };

    let execution_result = match state
        .market_data_client
        .execute_order(execution_request)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Order {} execution failed: {}", order.id, e);
            // Order remains Pending - could be retried later
            return Err(format!("Execution failed: {e}"));
        }
    };

    info!(
        "Order {} execution result: {:?}, filled: {}/{}, price: {:?}",
        order.id,
        execution_result.status,
        execution_result.filled_quantity,
        order.quantity,
        execution_result.average_price
    );

    // UPDATE ORDER WITH EXECUTION RESULTS
    let updated_order = match state
        .order_client
        .update_order_execution(
            order.id,
            execution_result.status.clone(),
            execution_result.filled_quantity,
            execution_result.average_price,
        )
        .await
    {
        Ok(updated) => updated,
        Err(e) => {
            error!(
                "Failed to update order {} with execution results: {}",
                order.id, e
            );
            return Err(format!("Failed to update order: {e}"));
        }
    };

    // POST-TRADE: UPDATE USER HOLDINGS AND BALANCE
    if matches!(
        execution_result.status,
        crate::clients::ExecutionStatus::FullyFilled
            | crate::clients::ExecutionStatus::PartiallyFilled
    ) {
        if let Some(exec_price) = execution_result.average_price {
            let quantity_change = match order_request.order_side {
                OrderSide::Buy => execution_result.filled_quantity as i64,
                OrderSide::Sell => -(execution_result.filled_quantity as i64),
            };

            // Calculate total transaction amount
            let transaction_amount = exec_price * execution_result.filled_quantity as f64;

            // Debit/Credit user account based on order side
            let funds_result = match order_request.order_side {
                OrderSide::Buy => {
                    state
                        .user_client
                        .withdraw_funds(user_id, transaction_amount)
                        .await
                }
                OrderSide::Sell => {
                    state
                        .user_client
                        .deposit_funds(user_id, transaction_amount)
                        .await
                }
            };

            if let Err(e) = funds_result {
                error!(
                    "Failed to {} user {} account for order {}: {}",
                    match order_request.order_side {
                        OrderSide::Buy => "debit",
                        OrderSide::Sell => "credit",
                    },
                    user_id,
                    order.id,
                    e
                );
                // Critical: funds operation failed - this is a serious issue
                // In production, you'd want to:
                // - Trigger a reconciliation process
                // - Alert operations team
                // - Possibly reverse the trade
                return Err(format!("Failed to update user funds: {e}"));
            }

            info!(
                "{} ${:.2} {} user {} for {} order {}",
                match order_request.order_side {
                    OrderSide::Buy => "Debited",
                    OrderSide::Sell => "Credited",
                },
                transaction_amount,
                match order_request.order_side {
                    OrderSide::Buy => "from",
                    OrderSide::Sell => "to",
                },
                user_id,
                match order_request.order_side {
                    OrderSide::Buy => "buy",
                    OrderSide::Sell => "sell",
                },
                order.id
            );

            // Update holdings
            if let Err(e) = state
                .user_client
                .update_holdings(user_id, &order_request.symbol, quantity_change, exec_price)
                .await
            {
                error!(
                    "Failed to update holdings for user {} after order {}: {}",
                    user_id, order.id, e
                );
                return Err(format!("Failed to update holdings: {e}"));
            }

            info!(
                "Updated holdings for user {}: {} {} shares @ ${}",
                user_id, quantity_change, order_request.symbol, exec_price
            );
        }
    }

    info!("Order {} completed successfully", updated_order.id);
    Ok(())
}
