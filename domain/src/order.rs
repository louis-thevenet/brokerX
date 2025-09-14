use in_memory_adapter::InMemoryRepo;

pub struct Order {
    pub id: String,
    pub symbol: String,
    pub quantity: u64,
}

pub type OrderId = u32;

pub type OrderRepo = InMemoryRepo<Order, OrderId>;
