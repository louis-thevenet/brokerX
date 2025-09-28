use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::user::UserId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64, // Average cost per share
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub user_id: UserId,
    pub holdings: HashMap<String, Holding>, // Symbol -> Holding
    pub total_value: f64, // Current market value (would be calculated with real-time prices)
    pub total_cost: f64,  // Total cost basis
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Portfolio {
    #[must_use]
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            holdings: HashMap::new(),
            total_value: 0.0,
            total_cost: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }

    #[must_use]
    pub fn get_holdings_list(&self) -> Vec<&Holding> {
        self.holdings.values().collect()
    }

    #[must_use]
    pub fn get_total_gain_loss(&self) -> f64 {
        self.total_value - self.total_cost
    }

    #[must_use]
    pub fn get_gain_loss_percentage(&self) -> f64 {
        if self.total_cost == 0.0 {
            0.0
        } else {
            (self.get_total_gain_loss() / self.total_cost) * 100.0
        }
    }
}
