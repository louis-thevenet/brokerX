use database_adapter::db::Repository;
use mfa_adapter::{EmailConfig, EmailOtpProvider, mfa::MfaService};
use tracing::info;

use crate::{
    order::{Order, OrderId, OrderRepo, OrderRepoExt, OrderSide, OrderStatus, OrderType},
    order_processing::ProcessingPool,
    pre_trade::{PreTradeError, PreTradeValidator},
    user::{UserId, UserRepo, UserRepoExt},
};

#[derive(Debug)]
pub struct BrokerX {
    pub mfa_service: MfaService<EmailOtpProvider>,
    pre_trade_validator: PreTradeValidator,
    processing_pool: ProcessingPool,
}

impl BrokerX {
    pub async fn new() -> Self {
        Self::with_thread_count(4).await
    }

    pub async fn with_thread_count(num_threads: usize) -> Self {
        let order_processing_pool = ProcessingPool::new(num_threads).await;
        BrokerX {
            mfa_service: MfaService::new(EmailOtpProvider::new(
                EmailConfig::from_env().expect("Email config creation failed"),
            )),
            pre_trade_validator: PreTradeValidator::with_default_config(),
            processing_pool: order_processing_pool,
        }
    }

    /// Create a test-friendly BrokerX instance that doesn't require environment variables
    /// and uses unique table names to avoid conflicts in parallel tests
    pub async fn new_for_testing() -> Self {
        Self::new_for_testing_with_thread_count(1).await
    }

    /// Create a test-friendly BrokerX instance with specified thread count
    pub async fn new_for_testing_with_thread_count(num_threads: usize) -> Self {
        let order_processing_pool = ProcessingPool::new_for_testing(num_threads).await;
        BrokerX {
            mfa_service: MfaService::new(EmailOtpProvider::new_for_testing()),
            pre_trade_validator: PreTradeValidator::with_default_config(),
            processing_pool: order_processing_pool,
        }
    }
    #[must_use]
    pub async fn get_user_repo(&self) -> UserRepo {
        self.processing_pool
            .shared_state
            .lock()
            .await
            .user_repo
            .clone()
    }
    #[must_use]
    pub async fn get_order_repo(&self) -> OrderRepo {
        self.processing_pool
            .shared_state
            .lock()
            .await
            .order_repo
            .clone()
    }
    pub async fn start_order_processing(&self) {
        self.processing_pool.start().await;
    }

    pub async fn stop_order_processing(&self) {
        self.processing_pool.stop().await;
    }

    /// Get orders for a specific user
    /// # Errors  
    /// Returns `DbError` if the database operation fails
    pub async fn get_orders_for_user(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<(OrderId, Order)>, database_adapter::db::DbError> {
        let shared_state = self.processing_pool.shared_state.lock().await;
        shared_state.order_repo.get_orders_for_user(user_id).await
    }

    /// Creates an order after performing pre-trade checks.
    /// # Errors
    /// Returns `PreTradeError` if any pre-trade validation fails.
    /// # Panics
    /// Panics if the order repository fails to create the order.
    pub async fn create_order(
        &self,
        client_id: UserId,
        symbol: String,
        quantity: u64,
        order_side: OrderSide,
        order_type: OrderType,
    ) -> Result<OrderId, PreTradeError> {
        // Get user balance for pre-trade checks
        let user_balance = {
            let state = self.processing_pool.shared_state.lock().await;
            match state.user_repo.get(&client_id).await {
                Ok(Some(user)) => user.balance,
                Ok(None) => 0.0,
                Err(_) => 0.0,
            }
        };

        // Pre-trade validation
        self.pre_trade_validator.validate_order(
            &order_side,
            &order_type,
            &symbol,
            quantity,
            user_balance,
        )?;

        // Create order after validation passes
        let date = chrono::Utc::now();
        let order = Order {
            client_id,
            date,
            symbol,
            quantity,
            order_side,
            order_type,
            status: OrderStatus::Queued,
        };

        // Create order in the thread pool's repository
        let order_id = {
            let state = self.processing_pool.shared_state.lock().await;
            state
                .order_repo
                .create_order(order)
                .await
                .map_err(PreTradeError::DbError)?
        };

        info!("Pre-trade checks validated for {order_id}");

        // Submit to processing pool
        self.processing_pool.submit_order(order_id).await;

        Ok(order_id)
    }
    #[allow(clippy::missing_panics_doc)]
    pub async fn debug_populate(&self) {
        let user_count = {
            let state = self.processing_pool.shared_state.lock().await;
            state.user_repo.len().await.unwrap_or(0)
        };

        if user_count > 0 {
            return;
        }

        let id = {
            let state = self.processing_pool.shared_state.lock().await;
            state
                .user_repo
                .create_user(
                    String::from("test@test.com"),
                    String::from("aaaaaa"),
                    String::from("Test"),
                    String::from("User"),
                    1000.0,
                )
                .await
                .unwrap()
        };

        {
            let state = self.processing_pool.shared_state.lock().await;
            state.user_repo.verify_user_email(&id).await.unwrap();
        }

        tracing::info!("Test user {} created with empty portfolio", id);
    }
}

impl Drop for BrokerX {
    fn drop(&mut self) {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.stop_order_processing());
        });
    }
}
