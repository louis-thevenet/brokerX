use std::collections::VecDeque;

use rand::random;

use crate::{
    account::{AccountRepo, AccountRepoExt},
    order::{Order, OrderId, OrderRepo, OrderStatus},
};

#[derive(Debug)]
pub struct BrokerX {
    pub account_repo: AccountRepo,
    order_repo: OrderRepo,
    order_queue: VecDeque<OrderId>,
}

impl BrokerX {
    #[must_use]
    pub fn new() -> Self {
        BrokerX {
            account_repo: AccountRepo::new(),
            order_repo: OrderRepo::new(),
            order_queue: VecDeque::new(),
        }
    }
    pub fn debug_populate(&mut self) {
        let _id = self.account_repo.create_account(
            String::from("Test Test"),
            String::from("test@test.com"),
            String::from("test"),
            String::from("test"),
            1000.0,
        );
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
                OrderStatus::Filled { date } => println!("Shouldn't happen"),
                OrderStatus::Cancelled => println!("Shouldn't happen"),
                OrderStatus::Expired { date } => println!("Shouldn't happen"),
                OrderStatus::Rejected { date } => println!("Shouldn't happen"),
            }
        }
    }
}

impl Default for BrokerX {
    fn default() -> Self {
        Self::new()
    }
}
