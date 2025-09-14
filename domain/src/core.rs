use chrono::NaiveDateTime;

use crate::{
    account::{Account, AccountRepo},
    order::{Order, OrderRepo},
};

#[derive(Debug)]
pub struct BrokerX {
    account_repo: AccountRepo,
    order_repo: OrderRepo,
}

impl BrokerX {
    #[must_use]
    pub fn new() -> Self {
        BrokerX {
            account_repo: AccountRepo::new(),
            order_repo: OrderRepo::new(),
        }
    }
    pub fn debug_populate(&mut self) {
        self.account_repo.insert(
            0,
            Account::new(
                String::from("Test Account"),
                String::from("test@test.com"),
                0.0,
            ),
        );
        self.order_repo.insert(
            0,
            Order::new(
                0,
                20.0,
                chrono::Local::now().naive_local(),
                String::from("AAAA"),
                1,
            ),
        );
    }
}

impl Default for BrokerX {
    fn default() -> Self {
        Self::new()
    }
}
