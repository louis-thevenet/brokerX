use crate::models::{AuthError, Holding, User, UserId};
use color_eyre::Result;
use dashmap::DashMap;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

#[derive(Clone, Debug)]
pub struct UserRepository {
    pub pool: PgPool,
    pub table_name: String,
    pub auth_cache: Arc<DashMap<String, (UserId, String)>>,
}

impl UserRepository {
    pub fn new(pool: PgPool, table_name: String) -> Self {
        Self {
            pool,
            table_name,
            auth_cache: Arc::new(DashMap::new()),
        }
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
            .map_err(AuthError::UserRepo)?;
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
            let holdings: HashMap<String, Holding> =
                serde_json::from_value(holdings_json).unwrap_or_default();
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
            .map_err(AuthError::UserRepo)?;
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
            .map_err(AuthError::UserRepo)?;
        Ok(user_id)
    }

    async fn authenticate_user(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<UserId>, AuthError> {
        // Check cache first
        if let Some(entry) = self.auth_cache.get(email) {
            let (user_id, cached_hash) = entry.value();
            let expected_hash = format!("hash_{password}");
            if cached_hash == &expected_hash {
                return Ok(Some(*user_id));
            } else {
                return Ok(None);
            }
        }
        // Only fetch id, email, password_hash for authentication
        let query = format!(
            "SELECT id, email, password_hash FROM {} WHERE email = $1",
            self.table_name
        );
        let row = sqlx::query(&query)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(AuthError::UserRepo)?;
        if let Some(row) = row {
            let id: Option<UserId> = row.try_get("id").ok();
            let password_hash: String = row.try_get("password_hash").unwrap();
            let expected_hash = format!("hash_{password}");
            if password_hash == expected_hash {
                // Cache result for future
                if let Some(uid) = id {
                    self.auth_cache
                        .insert(email.to_string(), (uid, password_hash.clone()));
                }
                return Ok(id);
            }
            return Ok(None);
        }
        Ok(None)
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
                AuthError::UserRepo(e)
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
            let holdings: HashMap<String, Holding> =
                serde_json::from_value(holdings_json).unwrap_or_default();
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
        // Optimized: Single UPDATE query with balance check
        let query = format!(
            "UPDATE {} SET balance = balance - $1 WHERE id = $2 AND balance >= $1 RETURNING id",
            self.table_name
        );

        let result = sqlx::query(&query)
            .bind(amount)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AuthError::UserRepo)?;

        match result {
            Some(_) => Ok(()),
            None => {
                // Check if user exists or just insufficient funds
                let user_exists =
                    sqlx::query(&format!("SELECT id FROM {} WHERE id = $1", self.table_name))
                        .bind(user_id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(AuthError::UserRepo)?
                        .is_some();

                if user_exists {
                    Err(AuthError::NotEnoughMoneyError)
                } else {
                    Err(AuthError::UserNotFound)
                }
            }
        }
    }

    async fn deposit_to_user(&self, user_id: &UserId, amount: f64) -> Result<(), AuthError> {
        // Optimized: Single UPDATE query
        let query = format!(
            "UPDATE {} SET balance = balance + $1 WHERE id = $2 RETURNING id",
            self.table_name
        );

        let result = sqlx::query(&query)
            .bind(amount)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AuthError::UserRepo)?;

        match result {
            Some(_) => Ok(()),
            None => Err(AuthError::UserNotFound),
        }
    }

    async fn update_user_holding(
        &self,
        user_id: &UserId,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> Result<(), AuthError> {
        if quantity_change > 0 {
            // BUY: Add shares or create new holding
            let query = format!(
                r"
                UPDATE {} SET holdings = 
                    CASE 
                        WHEN holdings ? $2 THEN
                            -- Update existing holding: recalculate average cost
                            jsonb_set(
                                holdings,
                                ARRAY[$2],
                                jsonb_build_object(
                                    'symbol', $2::text,
                                    'quantity', ((holdings->$2->>'quantity')::bigint + $3),
                                    'average_cost', (
                                        ((holdings->$2->>'quantity')::bigint * (holdings->$2->>'average_cost')::float8 + 
                                         $3 * $4) / 
                                        ((holdings->$2->>'quantity')::bigint + $3)
                                    )
                                )
                            )
                        ELSE
                            -- Create new holding
                            holdings || jsonb_build_object($2, jsonb_build_object(
                                'symbol', $2::text,
                                'quantity', $3,
                                'average_cost', $4
                            ))
                    END
                WHERE id = $1
                RETURNING id
                ",
                self.table_name
            );

            let result = sqlx::query(&query)
                .bind(user_id)
                .bind(symbol)
                .bind(quantity_change)
                .bind(execution_price)
                .fetch_optional(&self.pool)
                .await
                .map_err(AuthError::UserRepo)?;

            if result.is_none() {
                return Err(AuthError::UserNotFound);
            }
        } else {
            // SELL: Decrease shares or remove holding
            let abs_change = (-quantity_change) as u64;

            // First check if user has enough shares (in single query)
            let query = format!(
                r#"
                UPDATE {} SET holdings = 
                    CASE 
                        WHEN (holdings->$2->>'quantity')::bigint > $3 THEN
                            -- Decrease quantity (keep holding)
                            jsonb_set(
                                holdings,
                                ARRAY[$2],
                                jsonb_build_object(
                                    'symbol', $2::text,
                                    'quantity', ((holdings->$2->>'quantity')::bigint - $3),
                                    'average_cost', (holdings->$2->>'average_cost')::float8
                                )
                            )
                        WHEN (holdings->$2->>'quantity')::bigint = $3 THEN
                            -- Remove holding (quantity becomes 0)
                            holdings - $2
                        ELSE
                            -- Insufficient shares - this will trigger error via RETURNING NULL
                            NULL
                    END
                WHERE id = $1 
                    AND holdings ? $2 
                    AND (holdings->$2->>'quantity')::bigint >= $3
                RETURNING id
                "#,
                self.table_name
            );

            let result = sqlx::query(&query)
                .bind(user_id)
                .bind(symbol)
                .bind(abs_change as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(AuthError::UserRepo)?;

            if result.is_none() {
                // Check if it's insufficient shares or user not found
                let user_exists =
                    sqlx::query(&format!("SELECT id FROM {} WHERE id = $1", self.table_name))
                        .bind(user_id)
                        .fetch_optional(&self.pool)
                        .await
                        .map_err(AuthError::UserRepo)?
                        .is_some();

                if user_exists {
                    return Err(AuthError::NotEnoughMoneyError);
                } else {
                    return Err(AuthError::UserNotFound);
                }
            }
        }

        Ok(())
    }
}
