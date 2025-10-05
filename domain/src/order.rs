use chrono::{DateTime, NaiveDateTime, Utc};
use database_adapter::db::DbError;
use database_adapter::db::PostgresRepo;
use database_adapter::db::Repository;
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::user::UserId;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
/// Represents the current status of an order
pub enum OrderStatus {
    /// Order has been cancelled by the user
    Cancelled,
    /// Order has been cancelled by the system
    Expired { date: NaiveDateTime },
    /// Order has been completely executed
    Filled { date: NaiveDateTime },
    /// The order has been sent to the exchange but hasn’t been executed yet.
    Pending,
    /// Order is in the process of being cancelled
    PendingCancel,
    /// Order has not yet been processed by the system
    Queued,
    /// Order has been rejected by the system
    Rejected { date: NaiveDateTime }, // TODO: reason?
}

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

pub type OrderId = Uuid;

pub type OrderRepo = PostgresRepo<Order, OrderId>;

#[allow(async_fn_in_trait)]
pub trait OrderRepoExt {
    async fn create_order(&self, order: Order) -> Result<OrderId, DbError>;
    async fn get_orders_for_user(&self, user_id: &UserId)
    -> Result<Vec<(OrderId, Order)>, DbError>;
}

impl OrderRepoExt for OrderRepo {
    async fn create_order(&self, order: Order) -> Result<OrderId, DbError> {
        let id = Uuid::new_v4();
        self.insert(id, order).await?;
        Ok(id)
    }

    async fn get_orders_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<(OrderId, Order)>, DbError> {
        self.find_all_by_field("client_id", &user_id.to_string())
            .await
    }
}
