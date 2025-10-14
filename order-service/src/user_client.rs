use crate::models::{OrderError, UserId, OrderSide};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};
use uuid::Uuid;

// User types for HTTP communication with user service
#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    InvalidCredentials,
    UserRepo(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
}

#[derive(Debug, Clone)]
pub struct UserClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticateRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticateResponse {
    pub authenticated: bool,
    pub message: String,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateHoldingsRequest {
    pub symbol: String,
    pub quantity_change: i64,
    pub execution_price: f64,
}

impl UserClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Get user by ID via HTTP API
    pub async fn get_user(&self, user_id: &UserId) -> Result<Option<User>, AuthError> {
        let url = format!("{}/users/{}", self.base_url, user_id);
        debug!("Calling user service: GET {}", url);

        match self.client.get(&url).send().await {
            Ok(response) => match response.status().as_u16() {
                200 => match response.json().await {
                    Ok(user) => Ok(Some(user)),
                    Err(_) => Err(AuthError::UserNotFound),
                },
                404 => Ok(None),
                _ => {
                    error!("User service returned status: {}", response.status());
                    Err(AuthError::UserNotFound)
                }
            },
            Err(e) => {
                error!("Failed to call user service: {}", e);
                Err(AuthError::UserNotFound)
            }
        }
    }

    /// Check if user exists via HTTP API
    pub async fn user_exists(&self, user_id: &UserId) -> Result<bool, AuthError> {
        match self.get_user(user_id).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Authenticate user via HTTP API
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<bool, AuthError> {
        let url = format!("{}/auth/login", self.base_url);
        let request = AuthenticateRequest {
            email: email.to_string(),
            password: password.to_string(),
        };

        debug!("Calling user service: POST {}", url);

        match self.client.post(&url).json(&request).send().await {
            Ok(response) => match response.status().as_u16() {
                200 => match response.json().await {
                    Ok(auth_response) => {
                        let auth_response: AuthenticateResponse = auth_response;
                        Ok(auth_response.authenticated)
                    }
                    Err(_) => Ok(false),
                },
                _ => {
                    error!("User service auth returned status: {}", response.status());
                    Ok(false)
                }
            },
            Err(e) => {
                error!("Failed to call user service for auth: {}", e);
                Err(AuthError::UserRepo(format!("Network error: {}", e)))
            }
        }
    }

    /// Get user by email via HTTP API
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let url = format!("{}/users/by-email/{}", self.base_url, email);
        debug!("Calling user service: GET {}", url);

        match self.client.get(&url).send().await {
            Ok(response) => match response.status().as_u16() {
                200 => match response.json().await {
                    Ok(user) => Ok(Some(user)),
                    Err(_) => Err(AuthError::UserNotFound),
                },
                404 => Ok(None),
                _ => {
                    error!("User service returned status: {}", response.status());
                    Err(AuthError::UserNotFound)
                }
            },
            Err(e) => {
                error!("Failed to call user service: {}", e);
                Err(AuthError::UserRepo(format!("Network error: {}", e)))
            }
        }
    }

    /// Update user holdings after order execution
    pub async fn update_holdings(
        &self,
        user_id: &UserId,
        symbol: &str,
        order_side: OrderSide,
        quantity: u64,
        execution_price: f64,
    ) -> Result<(), OrderError> {
        let url = format!("{}/users/{}/holdings", self.base_url, user_id);
        
        // For Buy orders, quantity_change is positive
        // For Sell orders, quantity_change is negative
        let quantity_change = match order_side {
            OrderSide::Buy => quantity as i64,
            OrderSide::Sell => -(quantity as i64),
        };
        
        let request = UpdateHoldingsRequest {
            symbol: symbol.to_string(),
            quantity_change,
            execution_price,
        };

        debug!(
            "Calling user service: PUT {} with quantity_change: {}, price: {}",
            url, quantity_change, execution_price
        );

        match self.client.put(&url).json(&request).send().await {
            Ok(response) => match response.status().as_u16() {
                200 => {
                    debug!("Successfully updated holdings for user {}", user_id);
                    Ok(())
                }
                _ => {
                    error!(
                        "User service holdings update returned status: {}",
                        response.status()
                    );
                    Err(OrderError::UserService(format!(
                        "Failed to update holdings: {}",
                        response.status()
                    )))
                }
            },
            Err(e) => {
                error!("Failed to call user service to update holdings: {}", e);
                Err(OrderError::UserService(format!(
                    "Network error updating holdings: {}",
                    e
                )))
            }
        }
    }

    /// Validate that a user exists and has sufficient balance
    pub async fn validate_user(&self, user_id: &UserId) -> Result<bool, OrderError> {
        match self.user_exists(user_id).await {
            Ok(exists) => Ok(exists),
            Err(_) => Err(OrderError::UserValidationFailed),
        }
    }
}
