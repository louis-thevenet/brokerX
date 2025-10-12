use crate::models::{Order, OrderError, OrderId, OrderStatus, UserId};
use crate::repository::{OrderRepo, OrderRepoExt};
use crate::user_client::UserClient;
use database_adapter::config::DatabaseConfig;
use database_adapter::db::Repository;
use std::sync::Arc;
use tracing::{debug, error};

pub struct OrderService {
    order_repo: OrderRepo,
    user_client: Arc<UserClient>,
}

impl OrderService {
    pub async fn new_with_config(
        table_name: &str,
        db_config: &DatabaseConfig,
        user_service_url: String,
    ) -> color_eyre::Result<Self> {
        let order_repo = OrderRepo::new_with_url(table_name, &db_config.database_url).await?;
        let user_client = Arc::new(UserClient::new(&user_service_url));

        Ok(Self {
            order_repo,
            user_client,
        })
    }

    pub async fn create_order(&self, order: Order) -> Result<OrderId, OrderError> {
        debug!("Processing new order for user: {}", order.client_id);

        // Validate user exists and has sufficient funds
        match self.user_client.validate_user(&order.client_id).await {
            Ok(true) => {}
            Ok(false) => return Err(OrderError::UserValidationFailed),
            Err(e) => {
                error!("User validation failed: {}", e);
                return Err(OrderError::UserService(e.to_string()));
            }
        }

        // Create the order
        self.order_repo.create_order(order).await
    }

    pub async fn get_order(&self, order_id: &OrderId) -> Result<Option<Order>, OrderError> {
        self.order_repo
            .get(order_id)
            .await
            .map_err(OrderError::Repository)
    }

    pub async fn get_orders_by_user(&self, user_id: &UserId) -> Result<Vec<Order>, OrderError> {
        self.order_repo.get_orders_by_user(user_id).await
    }

    pub async fn get_pending_orders(&self) -> Result<Vec<Order>, OrderError> {
        self.order_repo.get_pending_orders().await
    }

    pub async fn update_order_status(
        &self,
        order_id: &OrderId,
        status: OrderStatus,
    ) -> Result<(), OrderError> {
        self.order_repo.update_order_status(order_id, status).await
    }

    pub async fn process_order(&self, order_id: &OrderId) -> Result<(), OrderError> {
        let order = self
            .get_order(order_id)
            .await?
            .ok_or(OrderError::OrderNotFound)?;

        debug!("Processing order: {:?}", order);

        // Simplified order processing logic
        // In a real system, this would involve:
        // 1. Market data validation
        // 2. Risk checks
        // 3. Order matching
        // 4. Settlement

        // For demo purposes, we'll just mark orders as filled or rejected based on simple rules
        let new_status = if order.quantity > 0 && order.quantity <= 1000 {
            OrderStatus::Filled {
                date: chrono::Utc::now(),
            }
        } else {
            OrderStatus::Rejected {
                date: chrono::Utc::now(),
            }
        };

        self.update_order_status(order_id, new_status).await
    }
}
