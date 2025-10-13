use crate::repository::{UserRepo, UserRepoExt};
use crate::models::{User, UserId, AuthError};
use color_eyre::Result;
use database_adapter::config::DatabaseConfig;
use database_adapter::db::{DbError, PostgresRepo, Repository};

/// User service that provides API interface for user operations
#[derive(Debug, Clone)]
pub struct UserService {
    user_repo: UserRepo,
}

impl UserService {
    pub async fn new(table_name: &str) -> Result<Self, DbError> {
        let user_repo = PostgresRepo::new(table_name).await?;
        Ok(Self { user_repo })
    }

    /// Create `UserService` with dedicated user database
    pub async fn new_with_config(
        table_name: &str,
        db_config: &DatabaseConfig,
    ) -> Result<Self, DbError> {
        let user_repo = PostgresRepo::new_with_url(table_name, &db_config.database_url).await?;
        Ok(Self { user_repo })
    }

    /// Get user by ID - API endpoint
    pub async fn get_user(&self, user_id: &UserId) -> Result<Option<User>, DbError> {
        self.user_repo.get(user_id).await
    }

    /// Create a new user - API endpoint
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

    /// Authenticate user - API endpoint
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<Option<UserId>, AuthError> {
        self.user_repo.authenticate_user(email, password).await
    }

    /// Get user by email - API endpoint
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        self.user_repo.get_user_by_email(email).await
    }

    /// Verify user email - API endpoint
    pub async fn verify_user_email(&self, user_id: &UserId) -> Result<(), AuthError> {
        let mut user = self
            .user_repo
            .get(user_id)
            .await
            .map_err(AuthError::UserRepo)?
            .ok_or(AuthError::UserNotFound)?;

        user.is_verified = true;
        self.user_repo
            .update(*user_id, user)
            .await
            .map_err(AuthError::UserRepo)?;

        Ok(())
    }

    /// Update user balance - API endpoint for order processing
    pub async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        self.user_repo.withdraw_from_user(user_id, amount).await
    }

    /// Add funds to user account - API endpoint for order processing  
    pub async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        self.user_repo.deposit_to_user(user_id, amount).await
    }

    /// Update user holdings - API endpoint for portfolio updates
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
}
