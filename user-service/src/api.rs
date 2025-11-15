use crate::event_publisher::PublishEventRequest;
use crate::models::{AuthError, Holding, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub firstname: String,
    pub surname: String,
    #[serde(default = "default_balance")]
    pub initial_balance: f64,
}

fn default_balance() -> f64 {
    0.0
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub surname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DepositRequest {
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WithdrawRequest {
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateHoldingsRequest {
    #[schema(value_type = String)]
    pub order_id: Uuid,
    pub symbol: String,
    pub quantity_change: i64,
    pub execution_price: f64,
    pub order_side: String,
    pub order_type: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthenticateRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthenticateResponse {
    pub authenticated: bool,
    pub message: String,
    #[schema(value_type = Option<String>, format = Uuid)]
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ValidateTradeRequest {
    pub symbol: String,
    pub quantity: u64,
    pub order_side: String, // "buy" or "sell"
    pub estimated_price: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidateTradeResponse {
    pub can_trade: bool,
    pub available_balance: f64,
    pub available_shares: u64,
    pub estimated_cost: f64,
    pub balance_after_trade: f64,
    pub message: Option<String>,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "user-service".to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/v1/users/{id}",
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn get_user(
    State(state): State<AppState>,
    Path(user_id_str): Path<String>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&user_id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.user_service.get_user(&user_id).await {
        Ok(Some(user)) => Json::<User>(user).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Create a new user
#[utoipa::path(
    post,
    path = "/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = Uuid),
        (status = 400, description = "Invalid request data"),
        (status = 409, description = "User already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "users"
)]
pub async fn create_user(
    State(state): State<AppState>,
    payload: Result<Json<CreateUserRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let Json(payload) = match payload {
        Ok(json) => json,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Basic validation
    if !payload.email.contains('@') {
        return (StatusCode::BAD_REQUEST, "Invalid email format").into_response();
    }

    if payload.password.is_empty() || payload.firstname.is_empty() || payload.surname.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    // Check if user already exists
    if let Ok(Some(_)) = state.user_service.get_user_by_email(&payload.email).await {
        return (StatusCode::CONFLICT, "User already exists").into_response();
    }

    match state
        .user_service
        .create_user(
            payload.email,
            payload.password,
            payload.firstname,
            payload.surname,
            payload.initial_balance,
        )
        .await
    {
        Ok(user_id) => (StatusCode::CREATED, Json(user_id)).into_response(),
        Err(AuthError::UserAlreadyExists) => {
            (StatusCode::CONFLICT, "User already exists").into_response()
        }
        Err(AuthError::WeakPassword) => {
            (StatusCode::BAD_REQUEST, "Password too weak").into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Update user information
#[utoipa::path(
    put,
    path = "/v1/users/{id}",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    info!("Updating user with ID: {}", user_id);
    match state.user_service.get_user(&user_id).await {
        Ok(Some(mut user)) => {
            if let Some(email) = &payload.email {
                if !email.contains('@') {
                    return (StatusCode::BAD_REQUEST, "Invalid email format").into_response();
                }
                if email != &user.email
                    && let Ok(Some(_)) = state.user_service.get_user_by_email(email).await
                {
                    return (StatusCode::CONFLICT, "Email already in use").into_response();
                }
                debug!("Updating email for user {}: {}", user_id, email);
                user.email = email.to_string();
            }
            if let Some(firstname) = &payload.firstname {
                debug!("Updating firstname for user {}: {}", user_id, firstname);
                user.firstname = firstname.to_string();
            }
            if let Some(surname) = &payload.surname {
                debug!("Updating surname for user {}: {}", user_id, surname);
                user.surname = surname.to_string();
            }
            match state.user_service.update_user(&user_id, user.clone()).await {
                Ok(()) => {
                    info!("User {} updated successfully", user_id);
                    Json(user).into_response()
                }
                Err(e) => {
                    info!("Error updating user {}: {}", user_id, e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Deposit funds into user account
#[utoipa::path(
    post,
    path = "/v1/users/{id}/deposit",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User balance updated successfully", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID"),
    ),
    tag = "users"
)]
pub async fn deposit_funds(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<DepositRequest>,
) -> impl IntoResponse {
    if payload.amount <= 0.0 {
        return (StatusCode::BAD_REQUEST, "Deposit amount must be positive").into_response();
    }
    match state
        .user_service
        .deposit_to_user(&user_id, payload.amount)
        .await
    {
        Ok(()) => {
            // Return success without fetching user again (saves 1 DB query)
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Deposit successful",
                    "amount": payload.amount
                })),
            )
                .into_response()
        }
        Err(e) => match e {
            AuthError::UserNotFound => StatusCode::NOT_FOUND.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    }
}

/// Withdraw funds from user account
#[utoipa::path(
    post,
    path = "/v1/users/{id}/withdraw",
    request_body = WithdrawRequest,
    responses(
        (status = 200, description = "User balance updated successfully", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request data or insufficient funds"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID"),
    ),
    tag = "users"
)]
pub async fn withdraw_funds(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<WithdrawRequest>,
) -> impl IntoResponse {
    if payload.amount <= 0.0 {
        return (StatusCode::BAD_REQUEST, "Withdraw amount must be positive").into_response();
    }
    match state
        .user_service
        .withdraw_from_user(&user_id, payload.amount)
        .await
    {
        Ok(()) => {
            // Return success without fetching user again (saves 1 DB query)
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Withdrawal successful",
                    "amount": payload.amount
                })),
            )
                .into_response()
        }
        Err(e) => match e {
            AuthError::UserNotFound => StatusCode::NOT_FOUND.into_response(),
            AuthError::NotEnoughMoneyError => {
                (StatusCode::BAD_REQUEST, "Insufficient funds").into_response()
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    }
}

/// Get user holdings
#[utoipa::path(
    get,
    path = "/v1/users/{id}/holdings",
    responses(
        (status = 200, description = "Holdings retrieved successfully", body = Vec<Holding>),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn get_holdings(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    debug!("Getting holdings for user {}", user_id);

    match state.user_service.get_user(&user_id).await {
        Ok(Some(user)) => {
            let holdings: Vec<_> = user.holdings.values().cloned().collect();
            debug!("Found {} holdings for user {}", holdings.len(), user_id);
            Json(holdings).into_response()
        }
        Ok(None) => {
            debug!("User {} not found", user_id);
            (StatusCode::NOT_FOUND, "User not found").into_response()
        }
        Err(e) => {
            error!("Error getting holdings for user {}: {}", user_id, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Update user holdings after order execution
#[utoipa::path(
    post,
    path = "/v1/users/{id}/holdings",
    request_body = UpdateHoldingsRequest,
    responses(
        (status = 200, description = "Holdings updated successfully"),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn update_holdings(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateHoldingsRequest>,
) -> impl IntoResponse {
    info!(
        "Updating holdings for user {}: {} {} @ {}",
        user_id, payload.quantity_change, payload.symbol, payload.execution_price
    );

    if payload.execution_price <= 0.0 {
        return (StatusCode::BAD_REQUEST, "Execution price must be positive").into_response();
    }

    match state
        .user_service
        .update_user_holding(
            &user_id,
            &payload.symbol,
            payload.quantity_change,
            payload.execution_price,
        )
        .await
    {
        Ok(()) => {
            info!(
                "Updated holdings for user {}: {} {}",
                user_id, payload.quantity_change, payload.symbol
            );

            let event = PublishEventRequest {
                order_id: payload.order_id,
                user_id,
                event_type: "HOLDINGS_UPDATED".to_string(),
                symbol: payload.symbol.clone(),
                quantity: payload.quantity_change.unsigned_abs(),
                order_side: payload.order_side.clone(),
                order_type: payload.order_type.clone(),
                filled_quantity: Some(payload.quantity_change.unsigned_abs()),
                average_price: Some(payload.execution_price),
            };

            if let Err(e) = state.event_publisher.publish_event(event).await {
                error!("Failed to publish HOLDINGS_UPDATED event: {}", e);
            }

            StatusCode::OK.into_response()
        }
        Err(e) => {
            error!("Error updating holdings for user {}: {}", user_id, e);
            match e {
                AuthError::UserNotFound => {
                    (StatusCode::NOT_FOUND, "User not found").into_response()
                }
                AuthError::NotEnoughMoneyError => {
                    (StatusCode::BAD_REQUEST, "Insufficient shares to sell").into_response()
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    }
}

/// Get user by email
#[utoipa::path(
    get,
    path = "/v1/users/by-email/{email}",
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("email" = String, Path, description = "User email")
    ),
    tag = "users"
)]
pub async fn get_user_by_email(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> impl IntoResponse {
    info!("Fetching user by email: {}", email);
    match state.user_service.get_user_by_email(&email).await {
        Ok(Some(user)) => Json::<User>(user).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Verify user email
#[utoipa::path(
    post,
    path = "/v1/users/{id}/verify",
    responses(
        (status = 200, description = "Email verified successfully"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn verify_user_email(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.user_service.verify_user_email(&user_id).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(AuthError::UserNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// Authenticate user credentials
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = AuthenticateRequest,
    responses(
        (status = 200, description = "Authentication result", body = AuthenticateResponse),
        (status = 400, description = "Invalid request data"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
pub async fn authenticate_user(
    State(state): State<AppState>,
    Json(payload): Json<AuthenticateRequest>,
) -> impl IntoResponse {
    info!("Login attempt for {}", payload.email);
    match state
        .user_service
        .authenticate_user(&payload.email, &payload.password)
        .await
    {
        Ok(user_id_opt) => {
            let (authenticated, message, user_id) = if let Some(user_id) = user_id_opt {
                (true, "Authentication successful".to_string(), Some(user_id))
            } else {
                (false, "Invalid credentials".to_string(), None)
            };

            let response = AuthenticateResponse {
                authenticated,
                message: message.clone(),
                user_id,
            };
            info!("Authentication result for {}: {}", payload.email, message);
            Json(response).into_response()
        }
        Err(e) => {
            info!("Authentication error for {}: {}", payload.email, e);
            let response = AuthenticateResponse {
                authenticated: false,
                message: e.to_string(),
                user_id: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Validate if user can execute a trade (pre-trade check)
#[utoipa::path(
    post,
    path = "/v1/users/{id}/validate-trade",
    request_body = ValidateTradeRequest,
    responses(
        (status = 200, description = "Trade validation result", body = ValidateTradeResponse),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    tag = "users"
)]
pub async fn validate_trade(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ValidateTradeRequest>,
) -> impl IntoResponse {
    debug!(
        "Validating trade for user {}: {} {} @ {}",
        user_id, payload.quantity, payload.symbol, payload.estimated_price
    );

    // Get user
    let user = match state.user_service.get_user(&user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            error!("Error fetching user {}: {}", user_id, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let order_side = payload.order_side.to_lowercase();
    let estimated_cost = payload.quantity as f64 * payload.estimated_price;

    let (can_trade, message, available_shares, balance_after) = match order_side.as_str() {
        "buy" => {
            let available_balance = user.balance;
            let can_afford = available_balance >= estimated_cost;
            let balance_after = if can_afford {
                available_balance - estimated_cost
            } else {
                available_balance
            };

            let msg = if !can_afford {
                Some(format!(
                    "Insufficient funds: need ${estimated_cost:.2}, have ${available_balance:.2}"
                ))
            } else {
                None
            };

            (can_afford, msg, 0, balance_after)
        }
        "sell" => {
            let available_shares = user
                .holdings
                .get(&payload.symbol)
                .map(|h| h.quantity)
                .unwrap_or(0);

            let can_sell = available_shares >= payload.quantity;
            let balance_after = user.balance + estimated_cost;

            let msg = if !can_sell {
                Some(format!(
                    "Insufficient shares: need {}, have {}",
                    payload.quantity, available_shares
                ))
            } else {
                None
            };

            (can_sell, msg, available_shares, balance_after)
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid order_side: must be 'buy' or 'sell'",
            )
                .into_response();
        }
    };

    let response = ValidateTradeResponse {
        can_trade,
        available_balance: user.balance,
        available_shares,
        estimated_cost,
        balance_after_trade: balance_after,
        message,
    };

    Json(response).into_response()
}
