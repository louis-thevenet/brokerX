use crate::AppState;
use crate::auth::JwtService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedUser;
use crate::models::*;
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
            MfaChallengeResponse,
            VerifyMfaRequest,
            VerifyMfaResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "user", description = "User management endpoints"),
        (name = "orders", description = "Order management endpoints")
    )
)]
pub struct ApiDoc;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
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
///
/// Creates a new user account and automatically initiates email verification.
/// The user must complete MFA verification before they can log in.
#[utoipa::path(
    post,
    path = "/auth/register",
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

    // MFA challenge will be sent when user attempts to login
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
/// that must be verified with /auth/verify-login to complete authentication.
#[utoipa::path(
    post,
    path = "/auth/login",
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

    // ALWAYS send MFA challenge for all users (maximum security)
    info!("Initiating MFA challenge for user {}", user_id);
    let challenge_id = state.mfa_service.initiate_challenge(&user.email).await?;

    Ok(Json(LoginResponse {
        user_id,
        challenge_id,
        message: "Multi-factor authentication required. Please check your email for the verification code.".to_string(),
    }))
}

/// Verify login MFA challenge
///
/// Complete the login process by verifying the MFA code sent to the user's email.
/// All login attempts require MFA verification for maximum security.
#[utoipa::path(
    post,
    path = "/auth/verify-login",
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
    let user_email = state
        .mfa_service
        .verify_challenge(&verify_request.challenge_id, &verify_request.code)
        .await?;

    // Get user by email
    let user_id = state.user_client.get_user_id_by_email(&user_email).await?;

    // Check if user is already verified
    let user = state.user_client.get_user(user_id).await?;

    // Mark as verified if this is their first successful MFA (prevents re-verification)
    if !user.is_verified {
        state.user_client.verify_email(user_id).await?;
        info!("User {} marked as verified (first login)", user_id);
    } else {
        info!("User {} completed MFA (already verified)", user_id);
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
/// In a production system with token blacklisting, this would invalidate the token server-side.
#[utoipa::path(
    post,
    path = "/auth/logout",
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
    path = "/me",
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
    path = "/me",
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
    path = "/me/deposit",
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
#[utoipa::path(
    post,
    path = "/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = OrderResponse),
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
    // Validate order request
    if order_request.symbol.is_empty() {
        return Err(ApiError::BadRequest("Symbol is required".to_string()));
    }

    if order_request.quantity == 0 {
        return Err(ApiError::BadRequest(
            "Quantity must be greater than 0".to_string(),
        ));
    }

    // For limit orders, price is required
    if matches!(order_request.order_type, OrderType::Limit { .. }) && order_request.price.is_none()
    {
        return Err(ApiError::BadRequest(
            "Price is required for limit orders".to_string(),
        ));
    }

    let order = state
        .order_client
        .create_order(user_id, &order_request)
        .await?;

    Ok((StatusCode::CREATED, Json(order)))
}

/// Get user's orders with pagination and filtering
#[utoipa::path(
    get,
    path = "/orders",
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
    path = "/orders/{id}",
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
    path = "/orders/{id}",
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
    path = "/orders/{id}",
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
