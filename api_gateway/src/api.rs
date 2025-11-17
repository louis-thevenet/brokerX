use crate::AppState;
use crate::auth::JwtService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedUser;
use crate::models::{
    CreateOrderRequest, DepositRequest, ErrorResponse, HealthResponse, HoldingResponse,
    LoginRequest, LoginResponse, LogoutRequest, MarketDataSubscription, MarketPriceResponse,
    MarketPricesResponse, MessageResponse, MfaChallengeResponse, OrderAcceptedResponse,
    OrderResponse, OrderSide, OrderStatus, OrderType, PaginatedResponse, PaginationInfo,
    PaginationQuery, RegisterRequest, RegisterResponse, SubscriptionFrequency, SubscriptionRequest,
    SymbolsResponse, UpdateOrderRequest, UpdateUserRequest, UserResponse, VerifyMfaRequest,
    VerifyMfaResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use tracing::{debug, error, info};
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        register,
        login,
        verify_login_mfa,
        logout,
        get_me,
        update_me,
        deposit_funds,
        create_order,
        get_my_orders,
        get_order,
        update_order,
        cancel_order,
        get_market_price,
        get_market_prices,
        get_market_symbols,
        get_market_history,
        get_my_subscription,
        upsert_my_subscription,
        delete_my_subscription,
    ),
    components(
        schemas(
            HealthResponse,
            RegisterRequest,
            RegisterResponse,
            LoginRequest,
            LoginResponse,
            LogoutRequest,
            UserResponse,
            HoldingResponse,
            UpdateUserRequest,
            DepositRequest,
            CreateOrderRequest,
            UpdateOrderRequest,
            OrderResponse,
            OrderSide,
            OrderType,
            OrderStatus,
            PaginationQuery,
            PaginatedResponse<OrderResponse>,
            PaginationInfo,
            MessageResponse,
            OrderAcceptedResponse,
            MfaChallengeResponse,
            VerifyMfaRequest,
            VerifyMfaResponse,
            MarketPriceResponse,
            MarketPricesResponse,
            SymbolsResponse,
            ErrorResponse,
            MarketDataSubscription,
            SubscriptionRequest,
            SubscriptionFrequency,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "user", description = "User management endpoints"),
        (name = "orders", description = "Order management endpoints"),
        (name = "market", description = "Market data endpoints")
    ),
    info(
        title = "BrokerX API Gateway",
        version = "1.0.0",
        description = "Main API Gateway for BrokerX platform - handles authentication, routing, and API composition.",
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "http://api-gateway:8080", description = "Docker service")
    )
)]
pub struct ApiDoc;

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
        service: "api-gateway".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Register a new user
#[utoipa::path(
    post,
    path = "/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = RegisterResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "User already exists")
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    debug!("Register attempt for email: {}", request.email);

    // Validate request
    if request.email.is_empty() || request.password.is_empty() {
        return Err(ApiError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    // Create user in user service
    info!("Creating user for {}", request.email);
    let user_id = state.user_client.create_user(&request).await?;
    info!("User created with ID: {}", user_id);

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user_id.to_string(),
            message: "Registration successful. Please login to verify your email and complete the process."
                .to_string(),
            requires_verification: true,
        }),
    ))
}

/// Login and get JWT token
///
/// MFA is required for all login attempts. A challenge ID will be returned
/// that must be verified with /v1/auth/verify-login to complete authentication.
#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "MFA challenge sent", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("Login attempt for {}", request.email);

    // Authenticate user credentials
    let user_id = state
        .user_client
        .authenticate_user(&request.email, &request.password)
        .await
        .map_err(|e| {
            error!("Authentication failed for {}: {}", request.email, e);
            e
        })?;

    // Get user info
    let user = state.user_client.get_user(user_id).await?;

    // send MFA challenge
    info!("Initiating MFA challenge for user {}", user_id);
    let challenge_id = if state.config.mfa_bypass {
        info!("MFA bypass enabled, no email sent");
        state
            .mfa_service
            .initiate_challenge_no_email(&user.email)
            .await?
    } else {
        state.mfa_service.initiate_challenge(&user.email).await?
    };

    Ok(Json(LoginResponse {
        user_id,
        challenge_id,
        message: "Multi-factor authentication required. Please check your email for the verification code.".to_string(),
    }))
}

/// Verify login MFA challenge
///
/// Complete the login process by verifying the MFA code sent to the user's email.
#[utoipa::path(
    post,
    path = "/v1/auth/verify-login",
    request_body = VerifyMfaRequest,
    responses(
        (status = 200, description = "MFA verification successful, token issued", body = VerifyMfaResponse),
        (status = 400, description = "Invalid verification code or challenge"),
        (status = 404, description = "Challenge not found or expired")
    ),
    tag = "auth"
)]
pub async fn verify_login_mfa(
    State(state): State<AppState>,
    Json(verify_request): Json<VerifyMfaRequest>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Verifying MFA for challenge: {}",
        verify_request.challenge_id
    );

    if verify_request.challenge_id.is_empty() {
        return Err(ApiError::BadRequest("Challenge ID is required".to_string()));
    }

    // Verify the MFA challenge
    let user_email = if state.config.mfa_bypass {
        info!("MFA bypass enabled, skipping verification");
        state
            .mfa_service
            .verify_challenge_bypass(&verify_request.challenge_id)
            .await?
    } else {
        state
            .mfa_service
            .verify_challenge(&verify_request.challenge_id, &verify_request.code)
            .await?
    };

    // Get user by email (returns full user object, which includes the ID)
    let user = state.user_client.get_user_by_email(&user_email).await?;
    let user_id = user.id;

    if user.is_verified {
        info!("User {} completed MFA (already verified)", user_id);
    } else {
        state.user_client.verify_email(user_id).await?;
        info!("User {} marked as verified (first login)", user_id);
    }

    // Issue JWT token
    let jwt_service = JwtService::new(&state.config.jwt_secret, state.config.jwt_expiry_hours);
    let (token, expires_at) = jwt_service.create_token(user_id)?;

    Ok(Json(VerifyMfaResponse {
        token,
        user_id,
        expires_at,
        message: "Authentication successful. You are now logged in.".to_string(),
    }))
}

/// Logout
///
/// Client-side logout. The client should delete its token.
/// TODO: revoke token server-side ?
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful", body = MessageResponse)
    ),
    tag = "auth"
)]
pub async fn logout() -> ApiResult<impl IntoResponse> {
    Ok(Json(MessageResponse {
        message: "Logout successful".to_string(),
    }))
}

/// Get current user profile
#[utoipa::path(
    get,
    path = "/v1/me",
    responses(
        (status = 200, description = "User profile", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn get_me(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<impl IntoResponse> {
    let user = state.user_client.get_user(user_id).await?;
    Ok(Json(user))
}

/// Update current user profile
#[utoipa::path(
    put,
    path = "/v1/me",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn update_me(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(update_request): Json<UpdateUserRequest>,
) -> ApiResult<impl IntoResponse> {
    let user = state
        .user_client
        .update_user(user_id, &update_request)
        .await?;
    Ok(Json(user))
}

/// Deposit funds to user account
#[utoipa::path(
    post,
    path = "/v1/me/deposit",
    request_body = DepositRequest,
    responses(
        (status = 200, description = "Funds deposited successfully", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid deposit amount")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn deposit_funds(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(deposit_request): Json<DepositRequest>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "User {} depositing amount: {}",
        user_id, deposit_request.amount
    );
    if deposit_request.amount <= 0.0 {
        error!(
            "Invalid deposit amount: {} for user {}",
            deposit_request.amount, user_id
        );
        return Err(ApiError::BadRequest(
            "Deposit amount must be positive".to_string(),
        ));
    }

    let user = state
        .user_client
        .deposit_funds(user_id, deposit_request.amount)
        .await?;
    info!("Deposit successful for user {}", user_id);
    Ok(Json(user))
}

/// Create a new order
/// Submits order to the order execution orchestrator service for async processing.
#[utoipa::path(
    post,
    path = "/v1/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 202, description = "Order accepted for processing", body = OrderAcceptedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid order request")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "orders"
)]
pub async fn create_order(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(order_request): Json<CreateOrderRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("User {} creating order: {:?}", user_id, order_request);

    // Basic validation
    if order_request.symbol.is_empty() {
        error!("Order creation failed: Symbol is required");
        return Err(ApiError::BadRequest("Symbol is required".to_string()));
    }

    if order_request.quantity == 0 {
        error!("Order creation failed: Quantity must be greater than 0");
        return Err(ApiError::BadRequest(
            "Quantity must be greater than 0".to_string(),
        ));
    }

    if matches!(order_request.order_type, OrderType::Limit { .. }) && order_request.price.is_none()
    {
        error!("Order creation failed: Price is required for limit orders");
        return Err(ApiError::BadRequest(
            "Price is required for limit orders".to_string(),
        ));
    }

    // Step 1: Create order in order-service (persists as Pending)
    let order = state
        .order_client
        .create_order(user_id, &order_request)
        .await?;

    info!(
        "Order {} created in order-service for user {}",
        order.id, user_id
    );

    // Step 2: Submit to orchestrator for async processing (best effort)
    // If orchestrator is down, order remains Pending and will be recovered on startup
    match state
        .orchestrator_client
        .submit_order(user_id, &order_request, Some(order.id))
        .await
    {
        Ok(()) => {
            info!(
                "Order {} submitted to orchestrator for user {}",
                order.id, user_id
            );
        }
        Err(e) => {
            // Log the error but don't fail the request - order is already persisted
            error!(
                "Failed to submit order {} to orchestrator: {}. Order will be processed on orchestrator recovery.",
                order.id, e
            );
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(OrderAcceptedResponse {
            order_id: order.id,
            message: "Order accepted for processing".to_string(),
        }),
    ))
}

/// Get user's orders with pagination and filtering
#[utoipa::path(
    get,
    path = "/v1/orders",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of user orders", body = PaginatedResponse<OrderResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "orders"
)]
pub async fn get_my_orders(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Query(pagination): Query<PaginationQuery>,
) -> ApiResult<impl IntoResponse> {
    // Validate pagination parameters
    if pagination.limit > 100 {
        return Err(ApiError::BadRequest("Limit cannot exceed 100".to_string()));
    }

    if pagination.page == 0 {
        return Err(ApiError::BadRequest(
            "Page must be greater than 0".to_string(),
        ));
    }

    let orders = state
        .order_client
        .get_user_orders(user_id, &pagination)
        .await?;

    Ok(Json(orders))
}

/// Get specific order by ID
#[utoipa::path(
    get,
    path = "/v1/orders/{id}",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order details", body = OrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - order belongs to another user"),
        (status = 404, description = "Order not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "orders"
)]
pub async fn get_order(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let order = state.order_client.get_order(user_id, order_id).await?;
    Ok(Json(order))
}

/// Update an existing order
#[utoipa::path(
    put,
    path = "/v1/orders/{id}",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    request_body = UpdateOrderRequest,
    responses(
        (status = 200, description = "Order updated successfully", body = OrderResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - order belongs to another user"),
        (status = 404, description = "Order not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "orders"
)]
pub async fn update_order(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(order_id): Path<Uuid>,
    Json(update_request): Json<UpdateOrderRequest>,
) -> ApiResult<impl IntoResponse> {
    let order = state
        .order_client
        .update_order(user_id, order_id, &update_request)
        .await?;
    Ok(Json(order))
}

/// Cancel an order
#[utoipa::path(
    delete,
    path = "/v1/orders/{id}",
    params(
        ("id" = Uuid, Path, description = "Order ID")
    ),
    responses(
        (status = 200, description = "Order cancelled successfully", body = MessageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - order belongs to another user"),
        (status = 404, description = "Order not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "orders"
)]
pub async fn cancel_order(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    state.order_client.cancel_order(user_id, order_id).await?;

    Ok(Json(MessageResponse {
        message: "Order cancelled successfully".to_string(),
    }))
}

// ============================================================================
// Market Data Endpoints
// ============================================================================

/// Get current price for a specific symbol
///
/// Returns the current market price and OHLCV data for the requested symbol.
/// This endpoint is public and does not require authentication.
#[utoipa::path(
    get,
    path = "/v1/market/prices/{symbol}",
    params(
        ("symbol" = String, Path, description = "Stock symbol (e.g., AAPL, GOOGL)")
    ),
    responses(
        (status = 200, description = "Current price data"),
        (status = 404, description = "Symbol not found"),
        (status = 503, description = "Market data service unavailable")
    ),
    tag = "market"
)]
pub async fn get_market_price(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let price = state.market_data_client.get_price(&symbol).await?;
    Ok(Json(price))
}

/// Get current prices for multiple symbols
///
/// Returns current market prices for multiple symbols in a single request.
/// This endpoint is public and does not require authentication.
#[utoipa::path(
    get,
    path = "/v1/market/prices",
    params(
        ("symbols" = String, Query, description = "Comma-separated list of symbols (e.g., AAPL,GOOGL,MSFT)")
    ),
    responses(
        (status = 200, description = "List of current prices"),
        (status = 503, description = "Market data service unavailable")
    ),
    tag = "market"
)]
pub async fn get_market_prices(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    if let Some(symbols_str) = params.get("symbols") {
        // If symbols are specified, fetch only those
        let symbols: Vec<String> = symbols_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !symbols.is_empty() {
            debug!(
                "Fetching market prices for {} specific symbols",
                symbols.len()
            );
            let prices = state.market_data_client.get_prices(&symbols).await?;
            return Ok(Json(prices));
        }
    }

    // No symbols specified or empty list - fetch all prices
    info!("Fetching all market prices");
    let prices = state.market_data_client.get_all_prices().await?;
    Ok(Json(prices))
}

/// Get list of available trading symbols
///
/// Returns all symbols available for trading.
/// This endpoint is public and does not require authentication.
#[utoipa::path(
    get,
    path = "/v1/market/symbols",
    responses(
        (status = 200, description = "List of available symbols"),
        (status = 503, description = "Market data service unavailable")
    ),
    tag = "market"
)]
pub async fn get_market_symbols(State(state): State<AppState>) -> ApiResult<impl IntoResponse> {
    info!("Fetching available market symbols");
    let symbols = state.market_data_client.get_symbols().await?;
    Ok(Json(symbols))
}

/// Get historical prices for a symbol
///
/// Returns historical price data for charting and analysis.
#[utoipa::path(
    get,
    path = "/v1/market/prices/{symbol}/history",
    params(
        ("symbol" = String, Path, description = "Stock symbol (e.g., AAPL)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of historical records (default: 100)")
    ),
    responses(
        (status = 200, description = "Historical price data"),
        (status = 404, description = "Symbol not found"),
        (status = 503, description = "Market data service unavailable")
    ),
    tag = "market"
)]
pub async fn get_market_history(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let limit = params.get("limit").and_then(|s| s.parse().ok());

    info!(
        "Fetching historical prices for symbol: {} (limit: {:?})",
        symbol, limit
    );
    let history = state
        .market_data_client
        .get_historical_prices(&symbol, limit)
        .await?;
    Ok(Json(history))
}

// ============================================================================
// Market Data Subscription Endpoints
// ============================================================================

/// Get user's market data subscription
///
/// Returns the authenticated user's market data subscription settings.
#[utoipa::path(
    get,
    path = "/v1/me/subscription",
    responses(
        (status = 200, description = "User's subscription", body = MarketDataSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn get_my_subscription(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<impl IntoResponse> {
    info!("User {} fetching subscription", user_id);
    let subscription = state.user_client.get_subscription(user_id).await?;
    Ok(Json(subscription))
}

/// Create or update user's market data subscription
///
/// Subscribe to market data reports for specific symbols.
/// If a subscription already exists, it will be updated with the new settings.
#[utoipa::path(
    put,
    path = "/v1/me/subscription",
    request_body = SubscriptionRequest,
    responses(
        (status = 200, description = "Subscription updated", body = MarketDataSubscription),
        (status = 201, description = "Subscription created", body = MarketDataSubscription),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn upsert_my_subscription(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(request): Json<SubscriptionRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("User {} creating/updating subscription", user_id);

    if request.symbols.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one symbol is required".to_string(),
        ));
    }

    let subscription = state
        .user_client
        .upsert_subscription(user_id, &request)
        .await?;

    // Send immediate welcome notification with current market data
    if let Err(e) = send_immediate_subscription_notification(&state, user_id, &request).await {
        error!("Failed to send immediate subscription notification: {}", e);
        // Don't fail the request if notification fails, just log it
    } else {
        // Update last_sent_at timestamp after successful notification
        if let Err(e) = state
            .user_client
            .update_subscription_last_sent(user_id)
            .await
        {
            error!("Failed to update last_sent_at timestamp: {}", e);
        }
    }

    Ok(Json(subscription))
}

/// Delete user's market data subscription
///
/// Unsubscribe from all market data reports.
#[utoipa::path(
    delete,
    path = "/v1/me/subscription",
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Subscription not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn delete_my_subscription(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> ApiResult<impl IntoResponse> {
    info!("User {} deleting subscription", user_id);
    state.user_client.delete_subscription(user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Helper function to send immediate market data notification when user subscribes
async fn send_immediate_subscription_notification(
    state: &AppState,
    user_id: Uuid,
    request: &SubscriptionRequest,
) -> ApiResult<()> {
    info!("Fetching market data for immediate subscription notification");

    // Get user information
    let user = state.user_client.get_user(user_id).await?;

    // Fetch current market data for subscribed symbols
    let market_data_response = state
        .market_data_client
        .get_prices(&request.symbols)
        .await?;

    // Parse the response - it's a MarketPricesResponse with a "prices" field
    let market_prices: Vec<MarketPriceResponse> =
        if let Some(prices_array) = market_data_response.get("prices") {
            serde_json::from_value(prices_array.clone()).map_err(|e| {
                ApiError::InternalServerError(format!("Failed to parse market data: {e}"))
            })?
        } else {
            // Fallback: try to parse the whole response as an array
            serde_json::from_value(market_data_response).map_err(|e| {
                ApiError::InternalServerError(format!("Failed to parse market data: {e}"))
            })?
        };

    // Get user holdings if requested
    let holdings_with_prices = if request.include_holdings {
        match state.user_client.get_user(user_id).await {
            Ok(user_response) => {
                // Convert HashMap to Vec
                let holdings_vec: Vec<HoldingResponse> =
                    user_response.holdings.into_values().collect();

                // Fetch current prices for all holdings
                let symbols: Vec<String> = holdings_vec.iter().map(|h| h.symbol.clone()).collect();

                if !symbols.is_empty() {
                    match state.market_data_client.get_prices(&symbols).await {
                        Ok(prices_response) => {
                            // Parse the response - it's a MarketPricesResponse with a "prices" field
                            let prices: Vec<MarketPriceResponse> =
                                if let Some(prices_array) = prices_response.get("prices") {
                                    serde_json::from_value(prices_array.clone()).unwrap_or_default()
                                } else {
                                    serde_json::from_value(prices_response).unwrap_or_default()
                                };

                            // Build a price map
                            let price_map: std::collections::HashMap<String, f64> = prices
                                .into_iter()
                                .map(|p| (p.symbol.clone(), p.price))
                                .collect();

                            // Pair holdings with current prices
                            let holdings_with_prices: Vec<(HoldingResponse, f64)> = holdings_vec
                                .into_iter()
                                .filter_map(|h| price_map.get(&h.symbol).map(|&price| (h, price)))
                                .collect();

                            Some(holdings_with_prices)
                        }
                        Err(e) => {
                            error!("Failed to fetch holding prices: {}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Err(e) => {
                error!("Failed to fetch user holdings: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Send notification
    state
        .notification_helper
        .send_market_data_notification(
            user_id.to_string(),
            user.email,
            market_prices,
            holdings_with_prices,
            request.frequency.to_string(),
        )
        .await?;

    info!("Immediate subscription notification sent successfully");
    Ok(())
}
