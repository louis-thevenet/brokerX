use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub type OrderId = Uuid;
pub type UserId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum OrderType {
    Market,
    Limit(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled { filled_quantity: u64 },
    Filled { date: DateTime<Utc> },
    Rejected { date: DateTime<Utc> },
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
pub enum OrderError {
    OrderNotFound,
    UserValidationFailed,
    InsufficientFunds,
    InvalidOrder,
    Repository(database_adapter::db::DbError),
    UserService(String),
    Processing(String),
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderError::OrderNotFound => write!(f, "Order not found"),
            OrderError::UserValidationFailed => write!(f, "User validation failed"),
            OrderError::InsufficientFunds => write!(f, "Insufficient funds"),
            OrderError::InvalidOrder => write!(f, "Invalid order"),
            OrderError::Repository(e) => write!(f, "Repository error: {}", e),
            OrderError::UserService(e) => write!(f, "User service error: {}", e),
            OrderError::Processing(e) => write!(f, "Processing error: {}", e),
        }
    }
}

impl From<database_adapter::db::DbError> for OrderError {
    fn from(err: database_adapter::db::DbError) -> Self {
        OrderError::Repository(err)
    }
}
