use std::collections::VecDeque;

use rand::random;
use tracing::error;

use crate::{
    mfa_factory::{DefaultMfaService, MfaServiceFactory},
    order::{OrderId, OrderRepo, OrderStatus},
    user::{UserRepo, UserRepoExt},
};

#[derive(Debug)]
pub struct BrokerX {
    pub user_repo: UserRepo,
    pub mfa_service: DefaultMfaService,
    order_repo: OrderRepo,
    order_queue: VecDeque<OrderId>,
}

impl BrokerX {
    #[must_use]
    pub fn new() -> Self {
        BrokerX {
            user_repo: UserRepo::new(),
            mfa_service: MfaServiceFactory::create_email_mfa_service(),
            order_repo: OrderRepo::new(),
            order_queue: VecDeque::new(),
        }
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

    fn update_orders(&mut self) {
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
                OrderStatus::Filled { date } => error!("Shouldn't happen"),
                OrderStatus::Cancelled => error!("Shouldn't happen"),
                OrderStatus::Expired { date } => error!("Shouldn't happen"),
                OrderStatus::Rejected { date } => error!("Shouldn't happen"),
            }
        }
    }
}

impl Default for BrokerX {
    fn default() -> Self {
        Self::new()
    }
}
