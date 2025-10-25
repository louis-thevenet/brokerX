use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Order Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    PartiallyFilled {
        filled_quantity: u64,
    },
    Filled {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
    Rejected {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
    Cancelled {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    #[schema(value_type = String, format = Uuid)]
    pub user_id: Uuid,
    #[schema(example = "AAPL")]
    pub symbol: String,
    #[schema(example = 10)]
    pub quantity: u64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    /// Price is required for limit orders, ignored for market orders
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    #[schema(value_type = String, format = Uuid)]
    pub id: Uuid,
    #[schema(value_type = String, format = Uuid)]
    pub client_id: Uuid,
    #[schema(value_type = String)]
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrderRequest {
    pub quantity: Option<u64>,
    pub order_type: Option<OrderType>,
}
