use crate::models::{Order, OrderError, OrderId, OrderStatus, UserId};
use chrono::Utc;
use color_eyre::Result;
use database_adapter::db::{PostgresRepo, Repository};
use tracing::debug;

pub type OrderRepo = PostgresRepo<Order, OrderId>;

#[allow(async_fn_in_trait)]
pub trait OrderRepoExt {
    async fn create_order(&self, order: Order) -> Result<(OrderId, Order), OrderError>;
    async fn get_orders_by_user(
        &self,
        user_id: &UserId,
        page: u32,
        limit: u32,
    ) -> Result<Vec<(OrderId, Order)>, OrderError>;
    async fn count_orders_by_user(&self, user_id: &UserId) -> Result<i64, OrderError>;
    async fn get_pending_orders(&self) -> Result<Vec<(OrderId, Order)>, OrderError>;
    async fn update_order_status(
        &self,
        order_id: &OrderId,
        status: OrderStatus,
    ) -> Result<(), OrderError>;
    async fn update_order(
        &self,
        order_id: &OrderId,
        order: Order,
    ) -> Result<Order, OrderError>;
    async fn delete_order(&self, order_id: &OrderId) -> Result<(), OrderError>;
}

impl OrderRepoExt for OrderRepo {
    async fn create_order(&self, order: Order) -> Result<(OrderId, Order), OrderError> {
        let order_id = uuid::Uuid::new_v4();
        debug!("Creating order with ID: {}", order_id);

        self.insert(order_id, order.clone())
            .await
            .map_err(OrderError::Repository)?;

        Ok((order_id, order))
    }

    async fn get_orders_by_user(
        &self,
        user_id: &UserId,
        page: u32,
        limit: u32,
    ) -> Result<Vec<(OrderId, Order)>, OrderError> {
        debug!(
            "Getting orders for user: {} (page: {}, limit: {})",
            user_id, page, limit
        );

        // Use find_all_by_field to get orders for the user
        let user_id_str = user_id.to_string();
        let user_orders = self
            .find_all_by_field("client_id", &user_id_str)
            .await
            .map_err(OrderError::Repository)?;
        
        // The query already orders by date DESC
        
        // Apply pagination
        let offset = ((page - 1) * limit) as usize;
        let paginated: Vec<(OrderId, Order)> = user_orders
            .into_iter()
            .skip(offset)
            .take(limit as usize)
            .collect();

        Ok(paginated)
    }

    async fn count_orders_by_user(&self, user_id: &UserId) -> Result<i64, OrderError> {
        debug!("Counting orders for user: {}", user_id);

        let user_id_str = user_id.to_string();
        let orders = self
            .find_all_by_field("client_id", &user_id_str)
            .await
            .map_err(OrderError::Repository)?;

        Ok(orders.len() as i64)
    }

    async fn get_pending_orders(&self) -> Result<Vec<(OrderId, Order)>, OrderError> {
        debug!("Getting all pending orders");

        // Note: This needs to filter by status which is complex in JSONB
        // For now, we can use a workaround or implement a custom query
        // We'll use find_all_by_field with the status field
        let pending_orders = self
            .find_all_by_field("status", "Pending")
            .await
            .map_err(OrderError::Repository)?;

        Ok(pending_orders)
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

    async fn update_order(
        &self,
        order_id: &OrderId,
        order: Order,
    ) -> Result<Order, OrderError> {
        debug!("Updating order {}", order_id);

        // Verify order exists
        self.get(order_id)
            .await
            .map_err(OrderError::Repository)?
            .ok_or(OrderError::OrderNotFound)?;

        self.update(*order_id, order.clone())
            .await
            .map_err(OrderError::Repository)?;

        Ok(order)
    }

    async fn delete_order(&self, order_id: &OrderId) -> Result<(), OrderError> {
        debug!("Deleting order {}", order_id);

        self.remove(*order_id)
            .await
            .map_err(OrderError::Repository)?;

        Ok(())
    }
}
