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

    // Note: Order is already created by API Gateway in order-service
    // This endpoint just queues it for processing
    info!("Queueing order for processing for user {}", user_id);

    // Queue the order for processing (order is already created in order-service)
    let queue = state.order_queue.as_ref();
    if let Err(e) = queue.send((user_id, order.clone(), None)) {
        error!("Failed to enqueue order for processing: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to queue order",
                "message": e.to_string()
            })),
        )
            .into_response();
    }

    // Increment queue length metric
    state.metrics.increment_queue();

    // Return acceptance - we don't have the order_id here, API Gateway will provide it
    (
        StatusCode::ACCEPTED,
        Json(crate::models::OrderAcceptedResponse {
            order_id: uuid::Uuid::nil(), // Placeholder, API Gateway has the real ID
            message: "Order queued for processing".to_string(),
        }),
    )
        .into_response()
}

/// Processes orders concurrently for maximum throughput
pub async fn order_processor(
    mut rx: mpsc::UnboundedReceiver<(Uuid, CreateOrderRequest, Option<Uuid>)>,
    state: Arc<AppState>,
) {
    info!("Order processor task started - processing orders concurrently with limits");
    let mut received_count = 0;

    while let Some((user_id, order_request, existing_order_id)) = rx.recv().await {
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
            if let Err(e) =
                process_single_order(user_id, order_request, existing_order_id, &state_clone).await
            {
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
    existing_order_id: Option<Uuid>,
    state: &AppState,
) -> Result<(), String> {
    // FIND/CREATE ORDER FIRST (so we have an order_id to update if validation fails)
    let order_id = if let Some(id) = existing_order_id {
        // Order ID passed from recovery - use it
        info!("Processing existing order {} from recovery", id);
        id
    } else {
        // Normal flow: Order was created by API Gateway, find it
        // Get the most recent pending order for this user that matches the request
        info!(
            "Looking up pending order for user {} symbol {} quantity {}",
            user_id, order_request.symbol, order_request.quantity
        );

        // Try to find matching pending order (created by API Gateway)
        let orders = match state.order_client.get_pending_orders().await {
            Ok(orders) => orders,
            Err(e) => {
                error!("Failed to fetch pending orders: {}", e);
                // Fallback: create order if we can't fetch pending ones
                match state
                    .order_client
                    .create_order(user_id, &order_request)
                    .await
                {
                    Ok(o) => {
                        info!("Created fallback order {} with status Pending", o.id);
                        return Ok(()); // Early return to avoid duplicate processing
                    }
                    Err(e2) => {
                        error!("Fallback order creation also failed: {}", e2);
                        return Err(format!("Failed to fetch or create order: {e}"));
                    }
                }
            }
        };

        // Find matching order
        let matching_order = orders.iter().find(|o| {
            o.client_id == user_id
                && o.symbol == order_request.symbol
                && o.quantity == order_request.quantity
                && matches!(
                    (&o.order_side, &order_request.order_side),
                    (OrderSide::Buy, OrderSide::Buy) | (OrderSide::Sell, OrderSide::Sell)
                )
        });

        match matching_order {
            Some(order) => {
                info!("Found matching pending order {} for processing", order.id);
                order.id
            }
            None => {
                warn!(
                    "No matching pending order found, creating new one for user {}",
                    user_id
                );
                match state
                    .order_client
                    .create_order(user_id, &order_request)
                    .await
                {
                    Ok(o) => {
                        info!("Order {} created with status Pending", o.id);
                        o.id
                    }
                    Err(e) => {
                        error!("Order creation failed for user {}: {}", user_id, e);
                        return Err(format!("Order creation failed: {e}"));
                    }
                }
            }
        }
    };

    // PRE-TRADE VALIDATION (now that we have order_id)
    info!(
        "Running pre-trade validation for order {} (user {})",
        order_id, user_id
    );
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
            "Pre-trade validation failed for order {} (user {}): {}",
            order_id, user_id, error_msg
        );

        // Mark order as Failed/Rejected with the validation error
        if let Err(e) = state
            .order_client
            .update_order_execution(
                order_id,
                crate::clients::ExecutionStatus::Rejected,
                0,
                None,
                Some(error_msg.clone()),
            )
            .await
        {
            error!("Failed to mark order {} as Rejected: {}", order_id, e);
        } else {
            info!(
                "Order {} marked as Rejected due to validation error: {}",
                order_id, error_msg
            );
        }

        return Err(format!("Validation failed: {error_msg}"));
    }

    // Log any warnings
    for warning in &validation_result.warnings {
        warn!(
            "Pre-trade warning for order {} (user {}): {}",
            order_id, user_id, warning
        );
    }

    info!(
        "Pre-trade validation passed for order {} (user {})",
        order_id, user_id
    );

    // ORDER EXECUTION
    let execution_request = crate::clients::ExecuteOrderRequest {
        order_id: order_id,
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
            // This is a true service error (not a business rejection)
            error!("Order {} execution service error: {}", order_id, e);

            // Mark order as Rejected due to execution service failure
            if let Err(update_err) = state
                .order_client
                .update_order_execution(
                    order_id,
                    crate::clients::ExecutionStatus::Rejected,
                    0,
                    None,
                    Some(format!("Service error: {}", e)),
                )
                .await
            {
                error!(
                    "Failed to mark order {} as Rejected after service error: {}",
                    order_id, update_err
                );
            } else {
                info!(
                    "Order {} marked as Rejected due to service error: {}",
                    order_id, e
                );
            }

            return Err(format!("Execution service error: {e}"));
        }
    };

    info!(
        "Order {} execution result: {:?}, filled: {}/{}, price: {:?}",
        order_id,
        execution_result.status,
        execution_result.filled_quantity,
        order_request.quantity,
        execution_result.average_price
    );

    // UPDATE ORDER WITH EXECUTION RESULTS
    // Note: Rejection reasons come from the execution result message
    let rejection_reason = if matches!(
        execution_result.status,
        crate::clients::ExecutionStatus::Rejected
    ) {
        execution_result.message.clone()
    } else {
        None
    };

    let updated_order = match state
        .order_client
        .update_order_execution(
            order_id,
            execution_result.status.clone(),
            execution_result.filled_quantity,
            execution_result.average_price,
            rejection_reason,
        )
        .await
    {
        Ok(updated) => updated,
        Err(e) => {
            error!(
                "Failed to update order {} with execution results: {}",
                order_id, e
            );
            return Err(format!("Failed to update order: {e}"));
        }
    };

    // Handle rejected orders - this is a valid outcome, not an error
    if matches!(
        execution_result.status,
        crate::clients::ExecutionStatus::Rejected
    ) {
        info!(
            "Order {} rejected by market: {}",
            order_id,
            execution_result
                .message
                .as_deref()
                .unwrap_or("Unknown reason")
        );
        // Order is rejected - complete successfully without updating funds/holdings
        return Ok(());
    }

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
                    order_id,
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
                order_id
            );

            // Update holdings
            if let Err(e) = state
                .user_client
                .update_holdings(user_id, &order_request.symbol, quantity_change, exec_price)
                .await
            {
                error!(
                    "Failed to update holdings for user {} after order {}: {}",
                    user_id, order_id, e
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
