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
