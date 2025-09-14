use crate::{account::AccountRepo, order::OrderRepo};

pub struct BrokerX {
    account_repo: AccountRepo,
    order_repo: OrderRepo,
}

impl BrokerX {
    pub fn new() -> Self {
        BrokerX {
            account_repo: AccountRepo::new(),
            order_repo: OrderRepo::new(),
        }
    }
}
