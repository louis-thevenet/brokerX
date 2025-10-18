use core::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub type OrderId = Uuid;
pub type UserId = Uuid;

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
    PartiallyFilled { filled_quantity: u64 },
    Filled { date: DateTime<Utc> },
    Rejected { date: DateTime<Utc> },
    Cancelled { date: DateTime<Utc> },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Order {
    #[schema(value_type = String, format = Uuid)]
    pub client_id: UserId,
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
    /// Price at which the order was filled (for market orders or filled limit orders)
    pub filled_price: Option<f64>,
}

// User model for inter-service communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Option<UserId>,
    pub email: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
}
#[derive(Debug)]
pub enum DbError {
    SqlxError(sqlx::Error),
    SerdeError(serde_json::Error),
    TokioError(std::io::Error),
}
impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::SqlxError(e) => write!(f, "Database error: {e}"),
            DbError::SerdeError(e) => write!(f, "Serialization error: {e}"),
            DbError::TokioError(e) => write!(f, "Runtime error: {e}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        DbError::SqlxError(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        DbError::SerdeError(error)
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        DbError::TokioError(error)
    }
}
#[derive(Debug)]
pub enum OrderError {
    OrderNotFound,
    InvalidOrder,
    OrderCannotBeModified,
    OrderCannotBeCancelled,
    Unauthorized,
    Repository(DbError),
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderError::OrderNotFound => write!(f, "Order not found"),
            OrderError::InvalidOrder => write!(f, "Invalid order"),
            OrderError::OrderCannotBeModified => write!(f, "Order cannot be modified"),
            OrderError::OrderCannotBeCancelled => write!(f, "Order cannot be cancelled"),
            OrderError::Unauthorized => write!(f, "Unauthorized"),
            OrderError::Repository(e) => write!(f, "Repository error: {e}"),
        }
    }
}

impl From<DbError> for OrderError {
    fn from(err: DbError) -> Self {
        OrderError::Repository(err)
    }
}
