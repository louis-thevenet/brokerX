use std::collections::VecDeque;

use rand::random;
use tracing::{error, info};

use crate::{
    mfa_factory::{DefaultMfaService, MfaServiceFactory},
    order::{Order, OrderId, OrderRepo, OrderRepoExt, OrderSide, OrderStatus, OrderType},
    pre_trade::{PreTradeError, PreTradeValidator},
    user::{UserId, UserRepo, UserRepoExt},
};

#[derive(Debug)]
pub struct BrokerX {
    pub user_repo: UserRepo,
    pub mfa_service: DefaultMfaService,
    pub order_repo: OrderRepo,
    order_queue: VecDeque<OrderId>,
    pre_trade_validator: PreTradeValidator,
}

impl BrokerX {
    #[must_use]
    pub fn new() -> Self {
        BrokerX {
            user_repo: UserRepo::new(),
            mfa_service: MfaServiceFactory::create_email_mfa_service(),
            order_repo: OrderRepo::new(),
            order_queue: VecDeque::new(),
            pre_trade_validator: PreTradeValidator::with_default_config(),
        }
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

        let order_id = self.order_repo.create_order(order);
        info!("Pre-trade checks validated for {order_id}");

        // Add to processing queue
        self.order_queue.push_back(order_id);

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

    pub fn update_orders(&mut self) {
        if !self.order_queue.is_empty() {
            let id = self.order_queue.pop_front().unwrap();

            let order = self.order_repo.get_mut(&id).unwrap();
            match order.status {
                OrderStatus::Queued => {
                    // TODO: check balance based on limit
                    order.status = OrderStatus::Pending;
                    self.order_queue.push_back(id);
                }
                OrderStatus::Pending => {
                    let random = random::<u32>() % 3;
                    if random == 0 {
                        order.status = OrderStatus::Filled {
                            date: chrono::Utc::now().naive_local(),
                        };
                    } else if random == 1 {
                        let amount_executed = if order.quantity > 1 {
                            order.quantity / 2
                        } else {
                            1
                        };
                        order.status = OrderStatus::PartiallyFilled { amount_executed };
                        self.order_queue.push_back(id);
                    } else {
                        order.status = OrderStatus::Rejected {
                            date: chrono::Utc::now().naive_local(),
                        };
                    }
                }
                OrderStatus::PartiallyFilled { amount_executed: _ } => {
                    order.status = OrderStatus::Filled {
                        date: chrono::Utc::now().naive_local(),
                    }
                }
                OrderStatus::PendingCancel => order.status = OrderStatus::Cancelled,
                OrderStatus::Filled { date: _ } => {
                    error!("Shouldn't happen - order already filled")
                }
                OrderStatus::Cancelled => error!("Shouldn't happen - order already cancelled"),
                OrderStatus::Expired { date: _ } => {
                    error!("Shouldn't happen - order already expired")
                }
                OrderStatus::Rejected { date: _ } => {
                    error!("Shouldn't happen - order already rejected")
                }
            }
        }
    }
}

impl Default for BrokerX {
    fn default() -> Self {
        Self::new()
    }
}
