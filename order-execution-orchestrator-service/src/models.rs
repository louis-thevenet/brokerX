use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Order Models
// ============================================================================

/// Order side indicating whether this is a buy or sell order
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Buy order - purchase securities
    Buy,
    /// Sell order - sell securities
    Sell,
}

/// Order type indicating pricing strategy
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    /// Market order - execute at current market price
    Market,
    /// Limit order - execute at specified price or better
    Limit {
        #[schema(example = 150.50)]
        price: f64,
    },
}

/// Order status tracking the lifecycle of an order
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrderStatus {
    /// Order is pending execution
    Pending,
    /// Order has been partially filled
    PartiallyFilled {
        #[schema(example = 5)]
        filled_quantity: u64,
    },
    /// Order has been completely filled
    Filled {
        #[schema(value_type = String, example = "2025-10-27T10:30:00Z")]
        date: DateTime<Utc>,
    },
    /// Order was rejected during validation or execution
    Rejected {
        #[schema(value_type = String, example = "2025-10-27T10:30:00Z")]
        date: DateTime<Utc>,
    },
    /// Order was cancelled by user or system
    Cancelled {
        #[schema(value_type = String, example = "2025-10-27T10:30:00Z")]
        date: DateTime<Utc>,
    },
}

/// Request to create a new order
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    /// User ID placing the order
    #[schema(value_type = String, format = Uuid, example = "550e8400-e29b-41d4-a716-446655440000")]
    pub user_id: Uuid,
    /// Stock symbol (ticker)
    #[schema(example = "AAPL")]
    pub symbol: String,
    /// Number of shares to trade
    #[schema(example = 10, minimum = 1)]
    pub quantity: u64,
    /// Buy or sell
    pub order_side: OrderSide,
    /// Market or limit order
    pub order_type: OrderType,
    /// Price is required for limit orders, ignored for market orders
    #[schema(example = 150.50)]
    pub price: Option<f64>,
    /// Order ID (optional) - if provided, skip order lookup and use this ID directly
    /// This eliminates the expensive get_pending_orders() call
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Uuid, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub order_id: Option<Uuid>,
}

/// Complete order information returned by the service
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    /// Unique order identifier
    #[schema(value_type = String, format = Uuid, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    /// Client/User ID who placed the order
    #[schema(value_type = String, format = Uuid, example = "550e8400-e29b-41d4-a716-446655440000")]
    pub client_id: Uuid,
    /// Order creation timestamp
    #[schema(value_type = String, example = "2025-10-27T10:30:00Z")]
    pub date: DateTime<Utc>,
    /// Stock symbol
    #[schema(example = "AAPL")]
    pub symbol: String,
    /// Number of shares
    #[schema(example = 10)]
    pub quantity: u64,
    /// Current order status
    pub status: OrderStatus,
    /// Order type (market or limit)
    pub order_type: OrderType,
    /// Order side (buy or sell)
    pub order_side: OrderSide,
}

/// Request to update an existing order
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrderRequest {
    /// New quantity (if updating)
    #[schema(example = 15)]
    pub quantity: Option<u64>,
    /// New order type (if updating)
    pub order_type: Option<OrderType>,
}

/// Response when an order is accepted for processing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderAcceptedResponse {
    /// The order ID (may be nil if not yet assigned)
    #[schema(value_type = String, format = Uuid, example = "123e4567-e89b-12d3-a456-426614174000")]
    pub order_id: Uuid,
    /// Status message
    #[schema(example = "Order queued for processing")]
    pub message: String,
}
