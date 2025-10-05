use color_eyre::Result;
use database_adapter::db::DbError;
use database_adapter::db::PostgresRepo;
use database_adapter::db::Repository;
use mfa_adapter::MfaError;
use mfa_adapter::MfaProvider;
use mfa_adapter::mfa::MfaService;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::portfolio::Holding;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct User {
    pub id: Option<Uuid>,
    pub email: String,
    pub password_hash: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub holdings: HashMap<String, Holding>, // Symbol -> Holding
}

#[derive(Debug)]
pub struct NotEnoughMoneyError;

#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    InvalidPassword,
    UserAlreadyExists,
    WeakPassword,
    MfaRequired,
    MfaFailed(MfaError),
    NotVerified(UserId),
    UserRepo(DbError),
    NotEnoughMoneyError,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::InvalidPassword => write!(f, "Invalid password"),
            AuthError::UserAlreadyExists => write!(f, "User already exists"),
            AuthError::WeakPassword => write!(f, "Password is too weak"),
            AuthError::MfaRequired => write!(f, "Multi-factor authentication required"),
            AuthError::MfaFailed(err) => write!(f, "MFA failed: {err}"),
            AuthError::NotVerified(_) => write!(f, "User email not verified"),
            AuthError::UserRepo(err) => {
                write!(f, "User repository error: {err}")
            }
            AuthError::NotEnoughMoneyError => {
                write!(f, "Not enough money in account")
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl User {
    pub fn new(
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<Self, AuthError> {
        if password.len() < 6 {
            return Err(AuthError::WeakPassword);
        }

        Ok(Self {
            id: None,
            email,
            password_hash: Self::hash_password(&password),
            firstname,
            surname,
            balance: initial_balance,
            is_verified: false,
            created_at: chrono::Utc::now(),
            holdings: HashMap::new(),
        })
    }

    pub fn verify_password(&self, password: &str) -> bool {
        // In a real app, use bcrypt or similar
        // For now, we'll use a simple hash for demonstration
        self.password_hash == Self::hash_password(password)
    }
    pub fn update_password(&mut self, password: &str) -> Result<(), AuthError> {
        if password.len() < 6 {
            return Err(AuthError::WeakPassword);
        }
        self.password_hash = Self::hash_password(password);
        Ok(())
    }
    fn hash_password(password: &str) -> String {
        // Simple hash for demonstration - use bcrypt in production!
        format!("hash_{password}")
    }

    /// Deposit money into the user's account
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    /// Withdraw money from the user's account
    pub fn withdraw(&mut self, amount: f64) -> Result<(), NotEnoughMoneyError> {
        if self.balance < amount {
            return Err(NotEnoughMoneyError);
        }
        self.balance -= amount;
        Ok(())
    }

    /// Get the current balance
    #[must_use]
    pub fn get_balance(&self) -> f64 {
        self.balance
    }

    /// Mark the user as verified
    pub fn verify_email(&mut self) {
        self.is_verified = true;
    }

    /// Update a holding (buy or sell shares)
    pub fn update_holding(&mut self, symbol: &str, quantity_change: i64, price: f64) {
        let symbol = symbol.to_string();

        if let Some(holding) = self.holdings.get_mut(&symbol) {
            // Update existing holding
            let old_quantity = holding.quantity as i64;
            let new_quantity = old_quantity + quantity_change;

            if new_quantity <= 0 {
                // Remove holding if quantity becomes zero or negative
                self.holdings.remove(&symbol);
            } else {
                // Update holding with new average cost
                let old_total_cost = holding.average_cost * holding.quantity as f64;
                let new_cost = if quantity_change > 0 {
                    price * quantity_change as f64
                } else {
                    0.0 // For sells, don't add to cost basis
                };

                holding.quantity = new_quantity as u64;
                if new_quantity > old_quantity {
                    // Only update average cost when buying
                    holding.average_cost = (old_total_cost + new_cost) / holding.quantity as f64;
                }
                holding.last_updated = chrono::Utc::now();
            }
        } else if quantity_change > 0 {
            // Create new holding (only for buys)
            self.holdings.insert(
                symbol.clone(),
                Holding {
                    symbol: symbol.clone(),
                    quantity: quantity_change as u64,
                    average_cost: price,
                    last_updated: chrono::Utc::now(),
                },
            );
        }
    }

    /// Get all holdings as a list
    pub fn get_holdings_list(&self) -> Vec<&Holding> {
        self.holdings.values().collect()
    }

    /// Get portfolio value (total cost basis for now)
    pub fn get_portfolio_value(&self) -> f64 {
        self.holdings
            .values()
            .map(|h| h.average_cost * h.quantity as f64)
            .sum()
    }

    /// Get total gain/loss (currently 0 since we use cost as current price)
    pub fn get_total_gain_loss(&self) -> f64 {
        0.0 // Would calculate based on current prices vs cost basis
    }

    /// Get gain/loss percentage
    pub fn get_gain_loss_percentage(&self) -> f64 {
        0.0 // Would calculate based on current prices vs cost basis
    }
}

pub type UserId = Uuid;

pub type UserRepo = PostgresRepo<User, UserId>;

#[allow(async_fn_in_trait)]
pub trait UserRepoExt {
    async fn create_user(
        &self,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError>;

    async fn authenticate_user(&self, email: &str, password: &str) -> Result<bool, AuthError>;

    async fn initiate_mfa<P: MfaProvider>(
        &self,
        email: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<String, AuthError>;

    fn complete_mfa_authentication<P: MfaProvider>(
        &self,
        challenge_id: &str,
        code: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<bool, AuthError>;

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;
    async fn get_user_by_id(&self, user_id: &UserId) -> Result<Option<User>, AuthError>;

    async fn email_exists(&self, email: &str) -> Result<bool, AuthError>;
    async fn is_verified(&self, email: &str) -> Result<bool, AuthError>;

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError>;
    async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError>;
    async fn get_user_balance(&self, user_id: &UserId) -> Result<f64, AuthError>;

    async fn verify_user_email(&self, user_id: &UserId) -> Result<(), AuthError>;
    async fn is_user_verified(&self, user_id: &UserId) -> Result<bool, AuthError>;
}

impl UserRepoExt for UserRepo {
    async fn create_user(
        &self,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError> {
        // Check if email already exists
        if self.email_exists(&email).await? {
            return Err(AuthError::UserAlreadyExists);
        }

        let mut user = User::new(email, password, firstname, surname, initial_balance)?;
        let user_id = Uuid::new_v4();
        user.id = Some(user_id);
        self.insert(user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;
        Ok(user_id)
    }
    async fn authenticate_user(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        if let Some(user) = self.get_user_by_email(email).await? {
            if !user.is_verified {
                debug!("User {} not verified", email);
                return Err(AuthError::NotVerified(user.id.unwrap_or_default()));
            }
            Ok(user.verify_password(password))
        } else {
            Err(AuthError::UserNotFound)
        }
    }

    async fn initiate_mfa<P: MfaProvider>(
        &self,
        email: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<String, AuthError> {
        let user = match self.get_user_by_email(email).await? {
            Some(u) => u,
            None => return Err(AuthError::UserNotFound),
        };

        mfa_service
            .initiate_mfa(&user.email)
            .await
            .map_err(AuthError::MfaFailed)
    }

    fn complete_mfa_authentication<P: MfaProvider>(
        &self,
        challenge_id: &str,
        code: &str,
        mfa_service: &MfaService<P>,
    ) -> Result<bool, AuthError> {
        mfa_service
            .verify_mfa(challenge_id, code)
            .map_err(AuthError::MfaFailed)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        self.find_by_field("email", email)
            .await
            .map_err(AuthError::UserRepo)
    }

    async fn get_user_by_id(&self, user_id: &UserId) -> Result<Option<User>, AuthError> {
        self.get(user_id).await.map_err(AuthError::UserRepo)
    }

    async fn email_exists(&self, email: &str) -> Result<bool, AuthError> {
        let user = self.get_user_by_email(email).await?;
        Ok(user.is_some())
    }

    async fn is_verified(&self, email: &str) -> Result<bool, AuthError> {
        let user = self
            .get_user_by_email(email)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        Ok(user.is_verified)
    }

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;
        user.deposit(amount);
        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;
        Ok(())
    }
    async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;
        user.withdraw(amount)
            .map_err(|_e| AuthError::NotEnoughMoneyError)?;
        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;
        Ok(())
    }

    async fn get_user_balance(&self, user_id: &UserId) -> Result<f64, AuthError> {
        let user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;
        Ok(user.get_balance())
    }

    async fn verify_user_email(&self, user_id: &UserId) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;
        user.verify_email();
        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;
        Ok(())
    }
    async fn is_user_verified(&self, user_id: &UserId) -> Result<bool, AuthError> {
        let user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;
        Ok(user.is_verified)
    }
}
