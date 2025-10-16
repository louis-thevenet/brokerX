use crate::models::{Order, OrderError, OrderSide, OrderStatus, OrderType};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub client_id: Uuid,
    pub symbol: String,
    pub quantity: u64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrderRequest {
    pub quantity: Option<u64>,
    pub order_type: Option<OrderType>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrderExecutionRequest {
    pub status: Option<OrderStatus>,
    pub filled_price: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    pub id: Uuid,
    pub client_id: Uuid,
    pub date: chrono::DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    pub filled_price: Option<f64>,
}

impl From<(Uuid, Order)> for OrderResponse {
    fn from((id, order): (Uuid, Order)) -> Self {
        Self {
            id,
            client_id: order.client_id,
            date: order.date,
            symbol: order.symbol,
            quantity: order.quantity,
            status: order.status,
            order_type: order.order_type,
            order_side: order.order_side,
            filled_price: order.filled_price,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, IntoParams)]
pub struct GetOrdersQuery {
    pub client_id: Option<Uuid>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total_items: i64,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedOrdersResponse {
    pub data: Vec<OrderResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "order-service".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get order by ID
#[utoipa::path(
    get,
    path = "/v1/orders/{id}",
    responses(
        (status = 200, description = "Order found", body = OrderResponse),
        (status = 404, description = "Order not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    tag = "orders"
)]
pub async fn get_order(
    State(state): State<AppState>,
    Path(order_id_str): Path<String>,
) -> impl IntoResponse {
    let order_id = match Uuid::parse_str(&order_id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.order_service.get_order(&order_id).await {
        Ok(Some(order)) => Json(OrderResponse::from(order)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Create a new order
#[utoipa::path(
    post,
    path = "/v1/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = OrderResponse),
        (status = 400, description = "Invalid request data"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "orders"
)]
pub async fn create_order(
    State(state): State<AppState>,
    payload: Result<Json<CreateOrderRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    info!("Received create order request: {:?}", payload);
    let Json(payload) = match payload {
        Ok(json) => json,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Basic validation
    if payload.symbol.is_empty() || payload.quantity == 0 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Create the order
    let order = Order {
        client_id: payload.client_id,
        date: chrono::Utc::now(),
        symbol: payload.symbol,
        quantity: payload.quantity,
        order_side: payload.order_side,
        order_type: payload.order_type,
        status: OrderStatus::Pending,
        filled_price: None,
    };

    match state.order_service.create_order(order).await {
        Ok((order_id, order)) => {
            info!("Order created successfully: {}", order_id);
            (
                StatusCode::CREATED,
                Json(OrderResponse::from((order_id, order))),
            )
                .into_response()
        }
        Err(e) => {
            error!("Error creating order: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Get orders with optional filtering and pagination
#[utoipa::path(
    get,
    path = "/v1/orders",
    params(GetOrdersQuery),
    responses(
        (status = 200, description = "Orders retrieved successfully", body = PaginatedOrdersResponse),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error")
    ),
    tag = "orders"
)]
pub async fn get_orders(
    State(state): State<AppState>,
    Query(query): Query<GetOrdersQuery>,
) -> impl IntoResponse {
    info!(
        "Getting orders with query: client_id={:?}, page={}, limit={}",
        query.client_id, query.page, query.limit
    );

    // Validate pagination
    if query.limit > 100 {
        return (StatusCode::BAD_REQUEST, "Limit cannot exceed 100").into_response();
    }

    if query.page == 0 {
        return (StatusCode::BAD_REQUEST, "Page must be greater than 0").into_response();
    }

    // For now, we require client_id (user filtering)
    let client_id = match query.client_id {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "client_id is required").into_response(),
    };

    match state
        .order_service
        .get_orders_by_user(&client_id, query.page, query.limit)
        .await
    {
        Ok((orders, total)) => {
            info!(
                "Found {} orders for user {}, total: {}",
                orders.len(),
                client_id,
                total
            );
            let order_responses: Vec<OrderResponse> =
                orders.into_iter().map(OrderResponse::from).collect();
            let total_pages = ((total as f64) / (query.limit as f64)).ceil() as u32;

            Json(PaginatedOrdersResponse {
                data: order_responses,
                pagination: PaginationInfo {
                    page: query.page,
                    limit: query.limit,
                    total_items: total,
                    total_pages,
                },
            })
            .into_response()
        }
        Err(e) => {
            error!("Error getting orders: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Update an existing order
#[utoipa::path(
    put,
    path = "/v1/orders/{id}",
    request_body = UpdateOrderRequest,
    responses(
        (status = 200, description = "Order updated successfully", body = OrderResponse),
        (status = 400, description = "Invalid request data"),
        (status = 403, description = "Forbidden - cannot modify this order"),
        (status = 404, description = "Order not found"),
        (status = 409, description = "Order cannot be modified in current state"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    tag = "orders"
)]
pub async fn update_order(
    State(state): State<AppState>,
    Path(order_id_str): Path<String>,
    payload: Result<Json<UpdateOrderRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let order_id = match Uuid::parse_str(&order_id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let Json(payload) = match payload {
        Ok(json) => json,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Get the original order to extract client_id
    let (_, original_order) = match state.order_service.get_order(&order_id).await {
        Ok(Some(order)) => order,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    match state
        .order_service
        .update_order(
            &order_id,
            &original_order.client_id,
            payload.quantity,
            payload.order_type,
        )
        .await
    {
        Ok(updated_order) => Json(OrderResponse::from((order_id, updated_order))).into_response(),
        Err(OrderError::OrderNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(OrderError::Unauthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(OrderError::OrderCannotBeModified) => {
            (StatusCode::CONFLICT, "Order cannot be modified").into_response()
        }
        Err(OrderError::InvalidOrder) => {
            (StatusCode::BAD_REQUEST, "Invalid order data").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Cancel an order
#[utoipa::path(
    delete,
    path = "/v1/orders/{id}",
    responses(
        (status = 200, description = "Order cancelled successfully"),
        (status = 403, description = "Forbidden - cannot cancel this order"),
        (status = 404, description = "Order not found"),
        (status = 409, description = "Order cannot be cancelled in current state"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    tag = "orders"
)]
pub async fn cancel_order(
    State(state): State<AppState>,
    Path(order_id_str): Path<String>,
) -> impl IntoResponse {
    info!("Received request to cancel order: {}", order_id_str);
    let order_id = if let Ok(id) = Uuid::parse_str(&order_id_str) {
        id
    } else {
        error!("Invalid UUID format for order ID: {}", order_id_str);
        return StatusCode::BAD_REQUEST.into_response();
    };

    // Get the order to extract client_id
    let (_, order) = match state.order_service.get_order(&order_id).await {
        Ok(Some(order)) => order,
        Ok(None) => {
            error!("Order not found: {}", order_id);
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            error!("Error retrieving order {}: {:?}", order_id, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match state
        .order_service
        .cancel_order(&order_id, &order.client_id)
        .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(OrderError::OrderNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(OrderError::Unauthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(OrderError::OrderCannotBeCancelled) => {
            (StatusCode::CONFLICT, "Order cannot be cancelled").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Update order with execution results from market data service
#[utoipa::path(
    patch,
    path = "/v1/orders/{id}/execution",
    request_body = UpdateOrderExecutionRequest,
    responses(
        (status = 200, description = "Order execution updated", body = OrderResponse),
        (status = 404, description = "Order not found"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    tag = "orders"
)]
pub async fn update_order_execution(
    State(state): State<AppState>,
    Path(order_id_str): Path<String>,
    Json(payload): Json<UpdateOrderExecutionRequest>,
) -> impl IntoResponse {
    info!("Updating execution for order: {}", order_id_str);

    let order_id = if let Ok(id) = Uuid::parse_str(&order_id_str) {
        id
    } else {
        error!("Invalid UUID format for order ID: {}", order_id_str);
        return StatusCode::BAD_REQUEST.into_response();
    };

    // Get the order first
    let (_, mut order) = match state.order_service.get_order(&order_id).await {
        Ok(Some(order)) => order,
        Ok(None) => {
            error!("Order not found: {}", order_id);
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(e) => {
            error!("Error retrieving order {}: {:?}", order_id, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Update order with execution results
    order.filled_price = payload.filled_price;

    // Update status based on execution result
    if let Some(status) = payload.status {
        order.status = status;
    }

    // Save updated order using the service method
    match state
        .order_service
        .update_order_with_execution(&order_id, order.clone())
        .await
    {
        Ok(()) => {
            info!("Order {} execution updated successfully", order_id);
            Json(OrderResponse::from((order_id, order))).into_response()
        }
        Err(e) => {
            error!("Error updating order {} execution: {:?}", order_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
