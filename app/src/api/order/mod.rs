use axum::{Json, extract::Path, extract::State, http::StatusCode, response::IntoResponse};
use domain::Repository;
use domain::order::{Order, OrderSide, OrderStatus, OrderType};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use super::AppState;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOrderRequest {
    pub client_id: Uuid,
    pub symbol: String,
    pub quantity: u64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateOrderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OrderStatus>,
}

pub fn router(state: AppState) -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .with_state(state)
        .routes(routes!(get_orders, post_order))
        .routes(routes!(get_order, put_order, delete_order))
}

/// Get all orders
///
/// Get all orders in the system
#[utoipa::path(
    get,
    path = "/api/order",
    responses(
        (status = 200,description = "Orders found",body = Vec<Order>),
        (status = 500,description = "Internal server error")
    ),
    tag = super::ORDER_TAG
)]
async fn get_orders(State(state): State<AppState>) -> impl IntoResponse {
    match state
        .broker()
        .get_order_repo()
        .await
        .find_all_by_field("client_id", "")
        .await
    {
        Ok(orders) => Json(orders).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Get order by UUID
///
/// Get a specific order by their UUID
#[utoipa::path(
    get,
    path = "/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "order UUID")
    ),
    responses(
        (status = 200, description = "Order found", body = Order),
        (status = 404, description = "Order not found"),
        (status = 400, description = "Invalid UUID format")
    ),
    tag = super::ORDER_TAG
)]
async fn get_order(State(state): State<AppState>, Path(order_id): Path<Uuid>) -> impl IntoResponse {
    let order_repo = state.broker().get_order_repo().await;
    match order_repo.get(&order_id).await {
        Ok(Some(order)) => Json(order).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Update order status by UUID
///
/// Update an existing order's status (mainly for cancellation)
#[utoipa::path(
    put,
    path = "/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order UUID")
    ),
    request_body = UpdateOrderRequest,
    responses(
        (status = 200, description = "Order updated successfully", body = Order),
        (status = 404, description = "Order not found"),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::ORDER_TAG
)]
async fn put_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
    Json(payload): Json<UpdateOrderRequest>,
) -> impl IntoResponse {
    let order_repo = state.broker().get_order_repo().await;

    match order_repo.get(&order_id).await {
        Ok(Some(mut order)) => {
            // Update order status if provided
            if let Some(status) = payload.status {
                order.status = status;
            }

            match order_repo.insert(order_id, order.clone()).await {
                Ok(()) => Json(order).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Create a new order
///
/// Create a new order. All fields are required.
#[utoipa::path(
    post,
    path = "/",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = Order),
        (status = 400, description = "Invalid request data or pre-trade validation failed"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::ORDER_TAG
)]
async fn post_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderRequest>,
) -> impl IntoResponse {
    match state
        .broker()
        .create_order(
            payload.client_id,
            payload.symbol,
            payload.quantity,
            payload.order_side,
            payload.order_type,
        )
        .await
    {
        Ok(order_id) => {
            // Retrieve the created order to return it
            let order_repo = state.broker().get_order_repo().await;
            match order_repo.get(&order_id).await {
                Ok(Some(order)) => (StatusCode::CREATED, Json(order)).into_response(),
                Ok(None) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Order creation error: {}", e),
        )
            .into_response(),
    }
}

/// Cancel order by UUID
///
/// Cancel (delete) a specific order by its UUID
#[utoipa::path(
    delete,
    path = "/{order_id}",
    params(
        ("order_id" = Uuid, Path, description = "Order UUID")
    ),
    responses(
        (status = 200, description = "Order cancelled successfully", body = Order),
        (status = 404, description = "Order not found"),
        (status = 400, description = "Order cannot be cancelled"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::ORDER_TAG
)]
async fn delete_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> impl IntoResponse {
    let order_repo = state.broker().get_order_repo().await;

    match order_repo.get(&order_id).await {
        Ok(Some(mut order)) => {
            // Check if order can be cancelled
            match order.status {
                OrderStatus::Filled { .. }
                | OrderStatus::Cancelled
                | OrderStatus::Expired { .. }
                | OrderStatus::Rejected { .. } => {
                    return (StatusCode::BAD_REQUEST, "Order cannot be cancelled").into_response();
                }
                _ => {
                    // Update order status to cancelled
                    order.status = OrderStatus::Cancelled;

                    match order_repo.insert(order_id, order.clone()).await {
                        Ok(()) => Json(order).into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests;
