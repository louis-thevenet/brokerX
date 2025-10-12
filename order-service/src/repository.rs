use crate::models::{Order, OrderError, OrderId, OrderStatus, UserId};
use chrono::Utc;
use color_eyre::Result;
use database_adapter::db::{PostgresRepo, Repository};
use tracing::debug;

pub type OrderRepo = PostgresRepo<Order, OrderId>;

#[allow(async_fn_in_trait)]
pub trait OrderRepoExt {
    async fn create_order(&self, order: Order) -> Result<OrderId, OrderError>;
    async fn get_orders_by_user(&self, user_id: &UserId) -> Result<Vec<Order>, OrderError>;
    async fn get_pending_orders(&self) -> Result<Vec<Order>, OrderError>;
    async fn update_order_status(
        &self,
        order_id: &OrderId,
        status: OrderStatus,
    ) -> Result<(), OrderError>;
}

impl OrderRepoExt for OrderRepo {
    async fn create_order(&self, mut order: Order) -> Result<OrderId, OrderError> {
        let order_id = uuid::Uuid::new_v4();
        debug!("Creating order with ID: {}", order_id);

        self.insert(order_id, order)
            .await
            .map_err(OrderError::Repository)?;

        Ok(order_id)
    }

    async fn get_orders_by_user(&self, user_id: &UserId) -> Result<Vec<Order>, OrderError> {
        debug!("Getting orders for user: {}", user_id);

        // TODO: Implement proper user-based filtering in the database adapter
        // For now, this is a placeholder that returns empty vector
        // In a real implementation, this would be a database query with WHERE clause
        Ok(Vec::new())
    }

    async fn get_pending_orders(&self) -> Result<Vec<Order>, OrderError> {
        debug!("Getting all pending orders");

        // TODO: Implement proper status-based filtering in the database adapter
        // For now, this is a placeholder that returns empty vector
        // In a real implementation, this would filter by OrderStatus::Pending
        Ok(Vec::new())
    }

    async fn update_order_status(
        &self,
        order_id: &OrderId,
        status: OrderStatus,
    ) -> Result<(), OrderError> {
        debug!("Updating order {} status to {:?}", order_id, status);

        let mut order = self
            .get(order_id)
            .await
            .map_err(OrderError::Repository)?
            .ok_or(OrderError::OrderNotFound)?;

        order.status = status;

        self.update(*order_id, order)
            .await
            .map_err(OrderError::Repository)?;

        Ok(())
    }
}
