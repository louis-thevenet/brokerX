use chrono::{DateTime, NaiveDateTime};
use in_memory_adapter::InMemoryRepo;

use crate::account::AccountId;

#[derive(Debug)]
pub struct Order {
    pub client_id: AccountId,
    pub price_at_time: f64,
    pub date: NaiveDateTime,
    pub symbol: String,
    pub quantity: u64,
}

impl Order {
    pub fn new(
        client_id: AccountId,
        price_at_time: f64,
        date: NaiveDateTime,
        symbol: String,
        quantity: u64,
    ) -> Self {
        Self {
            client_id,
            price_at_time,
            date,
            symbol,
            quantity,
        }
    }
}

pub type OrderId = u32;

pub type OrderRepo = InMemoryRepo<Order, OrderId>;
