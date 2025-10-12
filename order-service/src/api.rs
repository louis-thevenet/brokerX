use crate::models::{Order, OrderError, OrderSide, OrderStatus, OrderType};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
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

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
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
    path = "/orders/{id}",
    responses(
        (status = 200, description = "Order found", body = Order),
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
        Ok(Some(order)) => Json(order).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Create a new order
#[utoipa::path(
    post,
    path = "/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = Uuid),
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
    };

    match state.order_service.create_order(order).await {
        Ok(order_id) => (StatusCode::CREATED, Json(order_id)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Get orders for a specific user
#[utoipa::path(
    get,
    path = "/users/{user_id}/orders",
    responses(
        (status = 200, description = "Orders found", body = Vec<Order>),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    tag = "orders"
)]
pub async fn get_orders_for_user(
    State(state): State<AppState>,
    Path(user_id_str): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.order_service.get_orders_by_user(&user_id).await {
        Ok(orders) => Json(orders).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
