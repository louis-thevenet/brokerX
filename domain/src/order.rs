use chrono::{DateTime, NaiveDateTime, Utc};
use database_adapter::db::DbError;
use database_adapter::db::PostgresRepo;
use database_adapter::db::Repository;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::user::UserId;
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents the current status of an order
pub enum OrderStatus {
    /// Order has not yet been processed by the system
    Queued,
    /// The order has been sent to the exchange but hasn’t been executed yet.
    Pending,
    /// Only part of the order has been executed
    PartiallyFilled { amount_executed: u64 },
    /// Order has been completely executed
    Filled { date: NaiveDateTime },
    /// Order is in the process of being cancelled
    PendingCancel,
    /// Order has been cancelled by the user
    Cancelled,
    /// Order has been cancelled by the system
    Expired { date: NaiveDateTime },
    /// Order has been rejected by the system
    Rejected { date: NaiveDateTime }, // TODO: reason?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub client_id: UserId,
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}

pub type OrderId = Uuid;

pub type OrderRepo = PostgresRepo<Order, OrderId>;

pub trait OrderRepoExt {
    fn create_order(&mut self, order: Order) -> Result<OrderId, DbError>;
}

impl OrderRepoExt for OrderRepo {
    fn create_order(&mut self, order: Order) -> Result<OrderId, DbError> {
        let id = Uuid::new_v4();
        self.insert(id, order)?;
        Ok(id)
    }
}
