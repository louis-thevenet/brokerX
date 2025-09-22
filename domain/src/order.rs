use chrono::NaiveDateTime;
use in_memory_adapter::InMemoryRepo;
use uuid::Uuid;

use crate::user::UserId;
#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub enum IssuerType {
    Buyer,
    Seller,
}

#[derive(Debug)]
pub struct Order {
    pub client_id: UserId,
    pub date: NaiveDateTime,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub issuer: IssuerType,
    /// Optional limit price for limit orders
    pub limit: Option<f64>,
}

impl Order {}

pub type OrderId = Uuid;

pub type OrderRepo = InMemoryRepo<Order, OrderId>;

pub trait OrderRepoExt {}
