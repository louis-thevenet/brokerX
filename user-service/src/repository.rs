use crate::models::{AuthError, Holding, User, UserId};
use color_eyre::Result;
use database_adapter::db::{PostgresRepo, Repository};
use std::collections::HashMap;
use tracing::error;

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

    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<UserId>, AuthError>;

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError>;

    async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError>;

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError>;

    async fn update_user_holding(
        &self,
        user_id: &UserId,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> Result<(), AuthError>;
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
        // Check if user already exists
        if let Some(_) = self.get_user_by_email(&email).await? {
            return Err(AuthError::UserAlreadyExists);
        }

        // Validate password strength
        if password.len() < 8 {
            return Err(AuthError::WeakPassword);
        }

        // Check if email already exists
        if let Some(_) = self.get_user_by_email(&email).await? {
            return Err(AuthError::UserAlreadyExists);
        }

        let mut user = User {
            id: None,
            email,
            password_hash: format!("hash_{password}"), // TODO: proper hashing
            firstname,
            surname,
            balance: initial_balance,
            is_verified: false,
            created_at: chrono::Utc::now(),
            holdings: HashMap::new(),
        };

        let user_id = uuid::Uuid::new_v4();
        user.id = Some(user_id);
        self.insert(user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;
        Ok(user_id)
    }

    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<UserId>, AuthError> {
        let user = self
            .get_user_by_email(email)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        // Simple password check (TODO: proper verification)
        let expected_hash = format!("hash_{password}");
        if user.password_hash != expected_hash {
            return Ok(None);
        }

        // Note: We no longer check is_verified here
        // The API Gateway will handle MFA verification enforcement
        // This allows the gateway to send MFA challenges to unverified users

        Ok(user.id)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        match self.find_by_field("email", email).await {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Error fetching user by email {}: {:?}", email, e);
                Err(AuthError::UserRepo(e))
            }
        }
    }

    async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;

        if user.balance < amount {
            return Err(AuthError::NotEnoughMoneyError);
        }

        user.balance -= amount;
        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;

        Ok(())
    }

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;

        user.balance += amount;
        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;

        Ok(())
    }

    async fn update_user_holding(
        &self,
        user_id: &UserId,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> Result<(), AuthError> {
        let mut user = self
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;

        // Update holdings
        let holding = user
            .holdings
            .entry(symbol.to_string())
            .or_insert_with(|| Holding {
                symbol: symbol.to_string(),
                quantity: 0,
                average_cost: 0.0,
            });

        if quantity_change > 0 {
            // Buy: update average price
            let new_quantity = holding.quantity + quantity_change as u64;
            let total_cost = (holding.quantity as f64 * holding.average_cost)
                + (quantity_change as f64 * execution_price);
            holding.average_cost = total_cost / new_quantity as f64;
            holding.quantity = new_quantity;
        } else {
            // Sell: just reduce quantity
            let abs_change = (-quantity_change) as u64;
            if holding.quantity < abs_change {
                return Err(AuthError::NotEnoughMoneyError); // Not enough shares
            }
            holding.quantity -= abs_change;
        }

        // Remove holding if quantity becomes 0
        if holding.quantity == 0 {
            user.holdings.remove(symbol);
        }

        self.update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;

        Ok(())
    }
}
