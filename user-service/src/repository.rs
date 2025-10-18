use crate::models::{AuthError, Holding, User, UserId};
use color_eyre::Result;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::error;

#[derive(Clone, Debug)]
pub struct UserRepository {
    pub pool: PgPool,
    pub table_name: String,
}

impl UserRepository {
    pub fn new(pool: PgPool, table_name: String) -> Self {
        Self { pool, table_name }
    }

    pub async fn get_user_by_id(&self, user_id: &UserId) -> Result<Option<User>, AuthError> {
        let query = format!(
            "SELECT id, email, password_hash, firstname, surname, balance, is_verified, created_at, holdings FROM {} WHERE id = $1",
            self.table_name
        );
        let row = sqlx::query(&query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AuthError::UserRepo(e.into()))?;
        if let Some(row) = row {
            let id: Option<UserId> = row.try_get("id").ok();
            let email: String = row.try_get("email").unwrap();
            let password_hash: String = row.try_get("password_hash").unwrap();
            let firstname: String = row.try_get("firstname").unwrap();
            let surname: String = row.try_get("surname").unwrap();
            let balance: f64 = row.try_get("balance").unwrap();
            let is_verified: bool = row.try_get("is_verified").unwrap();
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").unwrap();
            let holdings_json: serde_json::Value = row.try_get("holdings").unwrap();
            let holdings: HashMap<String, Holding> = serde_json::from_value(holdings_json).unwrap_or_default();
            Ok(Some(User {
                id,
                email,
                password_hash,
                firstname,
                surname,
                balance,
                is_verified,
                created_at,
                holdings,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_user(&self, user_id: &UserId, user: &User) -> Result<(), AuthError> {
        let holdings_json = serde_json::to_value(&user.holdings).unwrap();
        let query = format!(
            "UPDATE {} SET email = $1, password_hash = $2, firstname = $3, surname = $4, balance = $5, is_verified = $6, holdings = $7 WHERE id = $8",
            self.table_name
        );
        sqlx::query(&query)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.firstname)
            .bind(&user.surname)
            .bind(user.balance)
            .bind(user.is_verified)
            .bind(holdings_json)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::UserRepo(e.into()))?;
        Ok(())
    }
}

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

impl UserRepoExt for UserRepository {
    async fn create_user(
        &self,
        email: String,
        password: String,
        firstname: String,
        surname: String,
        initial_balance: f64,
    ) -> Result<UserId, AuthError> {
        // Check if user already exists
        if self.get_user_by_email(&email).await?.is_some() {
            return Err(AuthError::UserAlreadyExists);
        }

        // Validate password strength
        if password.len() < 8 {
            return Err(AuthError::WeakPassword);
        }

        let user_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();
        let password_hash = format!("hash_{password}"); // TODO: proper hashing
        let holdings: HashMap<String, Holding> = HashMap::new();
        let holdings_json = serde_json::to_value(&holdings).unwrap();
        let query = format!(
            "INSERT INTO {} (id, email, password_hash, firstname, surname, balance, is_verified, created_at, holdings) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            self.table_name
        );
        sqlx::query(&query)
            .bind(user_id)
            .bind(&email)
            .bind(&password_hash)
            .bind(&firstname)
            .bind(&surname)
            .bind(initial_balance)
            .bind(false)
            .bind(now)
            .bind(holdings_json)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::UserRepo(e.into()))?;
        Ok(user_id)
    }

    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<UserId>, AuthError> {
        let user = self.get_user_by_email(email).await?.ok_or(AuthError::UserNotFound)?;
        let expected_hash = format!("hash_{password}");
        if user.password_hash != expected_hash {
            return Ok(None);
        }
        Ok(user.id)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let query = format!(
            "SELECT id, email, password_hash, firstname, surname, balance, is_verified, created_at, holdings FROM {} WHERE email = $1",
            self.table_name
        );
        let row = sqlx::query(&query)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!("Error fetching user by email {}: {:?}", email, e);
                AuthError::UserRepo(e.into())
            })?;
        if let Some(row) = row {
            let id: Option<UserId> = row.try_get("id").ok();
            let email: String = row.try_get("email").unwrap();
            let password_hash: String = row.try_get("password_hash").unwrap();
            let firstname: String = row.try_get("firstname").unwrap();
            let surname: String = row.try_get("surname").unwrap();
            let balance: f64 = row.try_get("balance").unwrap();
            let is_verified: bool = row.try_get("is_verified").unwrap();
            let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at").unwrap();
            let holdings_json: serde_json::Value = row.try_get("holdings").unwrap();
            let holdings: HashMap<String, Holding> = serde_json::from_value(holdings_json).unwrap_or_default();
            Ok(Some(User {
                id,
                email,
                password_hash,
                firstname,
                surname,
                balance,
                is_verified,
                created_at,
                holdings,
            }))
        } else {
            Ok(None)
        }
    }

    async fn withdraw_from_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self.get_user_by_id(user_id).await?.ok_or(AuthError::UserNotFound)?;
        if user.balance < amount {
            return Err(AuthError::NotEnoughMoneyError);
        }
        user.balance -= amount;
        self.update_user(user_id, &user).await?;
        Ok(())
    }

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        let mut user = self.get_user_by_id(user_id).await?.ok_or(AuthError::UserNotFound)?;
        user.balance += amount;
        self.update_user(user_id, &user).await?;
        Ok(())
    }

    async fn update_user_holding(
        &self,
        user_id: &UserId,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> Result<(), AuthError> {
        let mut user = self.get_user_by_id(user_id).await?.ok_or(AuthError::UserNotFound)?;
        let holding = user
            .holdings
            .entry(symbol.to_string())
            .or_insert_with(|| Holding {
                symbol: symbol.to_string(),
                quantity: 0,
                average_cost: 0.0,
            });
        if quantity_change > 0 {
            let new_quantity = holding.quantity + quantity_change as u64;
            let total_cost = (holding.quantity as f64 * holding.average_cost)
                + (quantity_change as f64 * execution_price);
            holding.average_cost = total_cost / new_quantity as f64;
            holding.quantity = new_quantity;
        } else {
            let abs_change = (-quantity_change) as u64;
            if holding.quantity < abs_change {
                return Err(AuthError::NotEnoughMoneyError);
            }
            holding.quantity -= abs_change;
        }
        if holding.quantity == 0 {
            user.holdings.remove(symbol);
        }
        self.update_user(user_id, &user).await?;
        Ok(())
    }
}
