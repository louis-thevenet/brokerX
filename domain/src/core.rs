use tracing::info;

use crate::{
    mfa_factory::{DefaultMfaService, MfaServiceFactory},
    order::{Order, OrderId, OrderRepoExt, OrderSide, OrderStatus, OrderType},
    order_processing::OrderProcessingPool,
    pre_trade::{PreTradeError, PreTradeValidator},
    user::{UserId, UserRepo, UserRepoExt},
};

#[derive(Debug)]
pub struct BrokerX {
    pub user_repo: UserRepo,
    pub mfa_service: DefaultMfaService,
    pre_trade_validator: PreTradeValidator,
    order_processing_pool: OrderProcessingPool,
}

impl BrokerX {
    #[must_use]
    pub fn new() -> Self {
        Self::with_thread_count(4)
    }

    #[must_use]
    pub fn with_thread_count(num_threads: usize) -> Self {
        let order_processing_pool = OrderProcessingPool::new(num_threads);

        BrokerX {
            user_repo: UserRepo::new(),
            mfa_service: MfaServiceFactory::create_email_mfa_service(),
            pre_trade_validator: PreTradeValidator::with_default_config(),
            order_processing_pool,
        }
    }

    pub fn start_order_processing(&self) {
        self.order_processing_pool.start();
    }

    pub fn stop_order_processing(&self) {
        self.order_processing_pool.stop();
    }
    pub fn create_order(
        &mut self,
        client_id: UserId,
        symbol: String,
        quantity: u64,
        order_side: OrderSide,
        order_type: OrderType,
    ) -> Result<OrderId, PreTradeError> {
        // Get user balance for pre-trade checks
        let user_balance = self
            .user_repo
            .get(&client_id)
            .map_or(0.0, |user| user.balance);

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
            let mut state = self.order_processing_pool.shared_state.lock().unwrap();
            state.order_repo.create_order(order)
        };

        info!("Pre-trade checks validated for {order_id}");

        // Submit to processing pool
        self.order_processing_pool.submit_order(order_id);

        Ok(order_id)
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn debug_populate(&mut self) {
        let id = self
            .user_repo
            .create_user(
                String::from("test@test.com"),
                String::from("aaaaaa"),
                String::from("Test"),
                String::from("User"),
                1000.0,
            )
            .unwrap();
        self.user_repo.verify_user_email(&id).unwrap();
    }

    /// Get an order by ID
    #[must_use]
    pub fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        self.order_processing_pool.get_order(order_id)
    }

    /// Get the current queue size (number of orders waiting to be processed)
    #[must_use]
    pub fn get_queue_size(&self) -> usize {
        self.order_processing_pool.get_queue_size()
    }

    /// Cancel an order (sets it to `PendingCancel` status)
    pub fn cancel_order(&self, order_id: &OrderId) -> Result<(), String> {
        self.order_processing_pool.cancel_order(order_id)
    }
}

impl Default for BrokerX {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BrokerX {
    fn drop(&mut self) {
        info!("BrokerX shutting down...");
        self.stop_order_processing();
    }
}
