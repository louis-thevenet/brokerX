use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub type UserId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    #[schema(value_type = String, format = Uuid)]
    pub id: Option<UserId>,
    pub email: String,
    pub password_hash: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub holdings: HashMap<String, Holding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Holding {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
}

/// Subscription frequency for market data reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionFrequency {
    Hourly,
    Daily,
    Weekly,
}

impl std::fmt::Display for SubscriptionFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionFrequency::Hourly => write!(f, "hourly"),
            SubscriptionFrequency::Daily => write!(f, "daily"),
            SubscriptionFrequency::Weekly => write!(f, "weekly"),
        }
    }
}

impl std::str::FromStr for SubscriptionFrequency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hourly" => Ok(SubscriptionFrequency::Hourly),
            "daily" => Ok(SubscriptionFrequency::Daily),
            "weekly" => Ok(SubscriptionFrequency::Weekly),
            _ => Err(format!("Invalid frequency: {s}")),
        }
    }
}

/// Market data subscription
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketDataSubscription {
    #[schema(value_type = String, format = Uuid)]
    pub id: Option<Uuid>,
    #[schema(value_type = String, format = Uuid)]
    pub user_id: UserId,
    pub symbols: Vec<String>,
    pub frequency: SubscriptionFrequency,
    pub include_holdings: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_sent_at: Option<DateTime<Utc>>,
}

/// Request to create or update a market data subscription
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionRequest {
    /// Symbols to subscribe to (e.g., ["AAPL", "GOOGL"])
    pub symbols: Vec<String>,
    /// Frequency of reports
    pub frequency: SubscriptionFrequency,
    /// Whether to include user's holdings in the report
    #[serde(default = "default_include_holdings")]
    pub include_holdings: bool,
}

fn default_include_holdings() -> bool {
    true
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Password too weak")]
    WeakPassword,
    #[error("Not enough money")]
    NotEnoughMoneyError,
    #[error("Repository error: {0}")]
    UserRepo(#[from] sqlx::Error),
}
