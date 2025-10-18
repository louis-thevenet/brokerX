use crate::models::{AuthError, User, UserId};
use crate::repository::{UserRepository, UserRepoExt};
use color_eyre::Result;

/// User service that provides API interface for user operations
#[derive(Debug, Clone)]
pub struct UserService {
    user_repo: UserRepository,
}

impl UserService {
    /// Create `UserService` with dedicated user database
    pub fn new(user_repo: UserRepository) -> Self {
        Self { user_repo }
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &UserId) -> Result<Option<User>, AuthError> {
        self.user_repo.get_user_by_id(user_id).await
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError> {
        self.user_repo
            .create_user(email, password, firstname, surname, initial_balance)
            .await
    }

    /// Authenticate user
    pub async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<UserId>, AuthError> {
        self.user_repo.authenticate_user(email, password).await
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        self.user_repo.get_user_by_email(email).await
    }

    /// Verify user email
    pub async fn verify_user_email(&self, user_id: &UserId) -> Result<(), AuthError> {
        let mut user = self
            .user_repo
            .get_user_by_id(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        user.is_verified = true;
        self.user_repo
            .update_user(user_id, &user)
            .await?;
        Ok(())
    }

    /// Add funds to user account
    pub async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        self.user_repo.deposit_to_user(user_id, amount).await
    }

    /// Withdraw funds from user account
    pub async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        self.user_repo.withdraw_from_user(user_id, amount).await
    }

    /// Update user holdings after order execution
    pub async fn update_user_holding(
        &self,
        user_id: &UserId,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> Result<(), AuthError> {
        self.user_repo
            .update_user_holding(user_id, symbol, quantity_change, execution_price)
            .await
    }

    /// Update user details
    pub async fn update_user(&self, user_id: &UserId, updated_user: User) -> Result<(), AuthError> {
        self.user_repo
            .update_user(user_id, &updated_user)
            .await
    }
}
