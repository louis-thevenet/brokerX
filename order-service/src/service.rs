use crate::config::DatabaseConfig;
use crate::models::{Order, OrderError, OrderId, OrderStatus, UserId};
use crate::repository::OrderRepository;
use sqlx::postgres::PgPoolOptions;
use tracing::debug;

pub struct OrderService {
    order_repo: OrderRepository,
}

impl OrderService {
    pub async fn new_with_config(
        _table_name: &str,
        db_config: &DatabaseConfig,
    ) -> color_eyre::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(100)
            .connect(&db_config.database_url)
            .await?;

        let order_repo = OrderRepository::new(pool);

        Ok(Self { order_repo })
    }

    pub async fn create_order(&self, order: Order) -> Result<(OrderId, Order), OrderError> {
        debug!("Creating order for user: {}", order.client_id);
        // No validation - API Gateway has already done pre-trade checks
        self.order_repo.create_order(order).await
    }

    pub async fn get_order(
        &self,
        order_id: &OrderId,
    ) -> Result<Option<(OrderId, Order)>, OrderError> {
        let order = self.order_repo.get_order_by_id(order_id).await?;

        Ok(order.map(|o| (*order_id, o)))
    }

    pub async fn get_orders_by_user(
        &self,
        user_id: &UserId,
        page: u32,
        limit: u32,
    ) -> Result<(Vec<(OrderId, Order)>, i64), OrderError> {
        let orders = self
            .order_repo
            .get_orders_by_user(user_id, page, limit)
            .await?;
        let total = self.order_repo.count_orders_by_user(user_id).await?;
        Ok((orders, total))
    }

    pub async fn get_pending_orders(&self) -> Result<Vec<(OrderId, Order)>, OrderError> {
        debug!("Fetching all pending orders");
        self.order_repo.get_pending_orders().await
    }

    pub async fn update_order(
        &self,
        order_id: &OrderId,
        user_id: &UserId,
        quantity: Option<u64>,
        order_type: Option<crate::models::OrderType>,
    ) -> Result<Order, OrderError> {
        // Get existing order
        let (_, mut order) = self
            .get_order(order_id)
            .await?
            .ok_or(OrderError::OrderNotFound)?;

        // Verify ownership
        if order.client_id != *user_id {
            return Err(OrderError::Unauthorized);
        }

        // Check if order can be modified
        match order.status {
            OrderStatus::Filled { .. }
            | OrderStatus::Rejected { .. }
            | OrderStatus::Cancelled { .. } => {
                return Err(OrderError::OrderCannotBeModified);
            }
            _ => {}
        }

        // Update fields
        if let Some(qty) = quantity {
            if qty == 0 {
                return Err(OrderError::InvalidOrder);
            }
            order.quantity = qty;
        }

        if let Some(ot) = order_type {
            order.order_type = ot;
        }

        // Save updated order
        self.order_repo
            .update_order(*order_id, order.clone())
            .await?;

        Ok(order)
    }

    pub async fn update_order_with_execution(
        &self,
        order_id: &OrderId,
        order: Order,
    ) -> Result<(), OrderError> {
        self.order_repo.update_order(*order_id, order).await?;
        Ok(())
    }

    pub async fn cancel_order(
        &self,
        order_id: &OrderId,
        user_id: &UserId,
    ) -> Result<(), OrderError> {
        // Get existing order
        let (_, order) = self
            .get_order(order_id)
            .await?
            .ok_or(OrderError::OrderNotFound)?;

        // Verify ownership
        if order.client_id != *user_id {
            return Err(OrderError::Unauthorized);
        }

        // Check if order can be cancelled
        match order.status {
            OrderStatus::Filled { .. }
            | OrderStatus::Rejected { .. }
            | OrderStatus::Cancelled { .. }
            | OrderStatus::PartiallyFilled { filled_quantity: _ } => {
                return Err(OrderError::OrderCannotBeCancelled);
            }
            OrderStatus::Pending => {}
        }

        let cancelled_status = OrderStatus::Cancelled {
            date: chrono::Utc::now(),
        };

        self.order_repo
            .update_order_status(order_id, cancelled_status)
            .await?;

        Ok(())
    }
}
