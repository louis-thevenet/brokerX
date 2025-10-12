use crate::order_domain::order::{Order, OrderId, OrderRepo, OrderRepoExt};
use color_eyre::Result;
use database_adapter::config::DatabaseConfig;
use database_adapter::db::{DbError, Repository};

/// Order service that provides API interface for order operations
/// This will eventually become a separate microservice
#[derive(Debug, Clone)]
pub struct OrderService {
    order_repo: OrderRepo,
}

impl OrderService {
    pub async fn new(table_name: &str) -> Result<Self, DbError> {
        let order_repo = OrderRepo::new(table_name).await?;
        Ok(Self { order_repo })
    }

    /// Create OrderService with dedicated order database
    pub async fn new_with_config(
        table_name: &str,
        db_config: &DatabaseConfig,
    ) -> Result<Self, DbError> {
        let order_repo = OrderRepo::new_with_url(table_name, &db_config.database_url).await?;
        Ok(Self { order_repo })
    }

    /// Create a new order - API endpoint
    pub async fn create_order(&self, order: Order) -> Result<OrderId, DbError> {
        self.order_repo.create_order(order).await
    }

    /// Get order by ID - API endpoint
    pub async fn get_order(&self, order_id: &OrderId) -> Result<Option<Order>, DbError> {
        self.order_repo.get(order_id).await
    }

    /// Update order - API endpoint
    pub async fn update_order(&self, order_id: OrderId, order: Order) -> Result<(), DbError> {
        self.order_repo.update(order_id, order).await
    }

    /// Get orders for user - API endpoint
    pub async fn get_orders_for_user(
        &self,
        user_id: &crate::order_domain::order::UserId,
    ) -> Result<Vec<(OrderId, Order)>, DbError> {
        self.order_repo.get_orders_for_user(user_id).await
    }

    /// Find orders by status - API endpoint  
    pub async fn find_orders_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<(OrderId, Order)>, DbError> {
        self.order_repo.find_all_by_field("status", status).await
    }
}
