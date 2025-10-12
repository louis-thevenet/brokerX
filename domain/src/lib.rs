pub mod core;
pub mod portfolio;
mod pre_trade;

// New modular domain structure
pub mod order_domain;
pub mod user_domain;

// Re-export for backward compatibility during transition
pub use order_domain::order;
pub use order_domain::order_processing;
pub use user_domain::user;

pub use database_adapter::db::Repository;
