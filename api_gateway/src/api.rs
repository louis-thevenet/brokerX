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
use utoipa::OpenApi;
use uuid::Uuid;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        register,
        login,
        get_me,
        update_me,
        verify_email,
        deposit_funds,
        create_order,
        get_my_orders,
        get_order,
        update_order,
        cancel_order
    ),
    components(
        schemas(
            HealthResponse,
            RegisterRequest,
            LoginRequest,
            LoginResponse,
            LogoutRequest,
            User,
            UpdateUserRequest,
            VerifyEmailRequest,
            DepositRequest,
            CreateOrderRequest,
            UpdateOrderRequest,
            Order,
            OrderSide,
            OrderType,
            OrderStatus,
            PaginationQuery,
            PaginatedResponse<Order>,
            PaginationInfo,
            MessageResponse
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
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = MessageResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "User already exists")
    ),
    tag = "auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate request
    if request.email.is_empty() || request.password.is_empty() {
        return Err(ApiError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    if request.initial_balance < 0.0 {
        return Err(ApiError::BadRequest(
            "Initial balance cannot be negative".to_string(),
        ));
    }

    let user_id = state.user_client.create_user(&request).await?;

    Ok((
        StatusCode::CREATED,
        Json(MessageResponse {
            message: format!("User registered successfully with ID: {}", user_id),
        }),
    ))
}

/// Login and get JWT token
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let user_id = state
        .user_client
        .authenticate_user(&request.email, &request.password)
        .await?;

    let jwt_service = JwtService::new(&state.config.jwt_secret, state.config.jwt_expiry_hours);
    let (token, expires_at) = jwt_service.create_token(user_id)?;

    Ok(Json(LoginResponse {
        token,
        user_id,
        expires_at,
    }))
}

/// Logout (invalidate token)
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
    // In a stateless JWT system, logout is typically handled client-side
    // For more security, you could implement a token blacklist
    Ok(Json(MessageResponse {
        message: "Logout successful".to_string(),
    }))
}

/// Get current user profile
#[utoipa::path(
    get,
    path = "/me",
    responses(
        (status = 200, description = "User profile", body = User),
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
        (status = 200, description = "User updated successfully", body = User),
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

/// Verify email address
#[utoipa::path(
    post,
    path = "/me/verify-email",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified successfully", body = MessageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid verification code")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "user"
)]
pub async fn verify_email(
    State(state): State<AppState>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(_verify_request): Json<VerifyEmailRequest>,
) -> ApiResult<impl IntoResponse> {
    state.user_client.verify_email(user_id).await?;

    Ok(Json(MessageResponse {
        message: "Email verified successfully".to_string(),
    }))
}

/// Deposit funds to user account
#[utoipa::path(
    post,
    path = "/me/deposit",
    request_body = DepositRequest,
    responses(
        (status = 200, description = "Funds deposited successfully", body = User),
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
    if deposit_request.amount <= 0.0 {
        return Err(ApiError::BadRequest(
            "Deposit amount must be positive".to_string(),
        ));
    }

    let user = state
        .user_client
        .deposit_funds(user_id, deposit_request.amount)
        .await?;
    Ok(Json(user))
}

/// Create a new order
#[utoipa::path(
    post,
    path = "/orders",
    request_body = CreateOrderRequest,
    responses(
        (status = 201, description = "Order created successfully", body = Order),
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
    if matches!(order_request.order_type, OrderType::Limit) && order_request.price.is_none() {
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
        (status = 200, description = "List of user orders", body = PaginatedResponse<Order>),
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
        (status = 200, description = "Order details", body = Order),
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
        (status = 200, description = "Order updated successfully", body = Order),
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
