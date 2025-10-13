// API Gateway Models - REST API DTOs
// These models represent the external API contract and follow REST API best practices
// They are separate from internal service models to allow independent evolution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ============================================================================
// Common Response Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
}

// ============================================================================
// Pagination
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total_items: u64,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

// ============================================================================
// Authentication & Authorization Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "SecurePassword123!")]
    pub password: String,
    #[schema(example = "John")]
    pub firstname: String,
    #[schema(example = "Doe")]
    pub surname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterResponse {
    #[schema(value_type = String, format = Uuid)]
    pub user_id: String,
    pub message: String,
    pub requires_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "SecurePassword123!")]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    /// User ID for this login attempt
    #[schema(value_type = String, format = Uuid)]
    pub user_id: Uuid,
    /// Challenge ID to use for MFA verification
    pub challenge_id: String,
    /// Message describing the next step
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    // Empty for now, token invalidation would happen on client side
    // In a production system, this might include a token to blacklist
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
}

// ============================================================================
// MFA Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MfaChallengeResponse {
    pub challenge_id: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Uuid)]
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyMfaRequest {
    pub challenge_id: String,
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyMfaResponse {
    pub token: String,
    #[schema(value_type = String, format = Uuid)]
    pub user_id: Uuid,
    #[schema(value_type = String)]
    pub expires_at: DateTime<Utc>,
    pub message: String,
}

// ============================================================================
// User Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    #[schema(value_type = String, format = Uuid)]
    pub id: Uuid,
    pub email: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
    pub holdings: std::collections::HashMap<String, HoldingResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HoldingResponse {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub surname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepositRequest {
    #[schema(example = 1000.0)]
    pub amount: f64,
}

// ============================================================================
// Order Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    PartiallyFilled {
        filled_quantity: u64,
    },
    Filled {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
    Rejected {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
    Cancelled {
        #[schema(value_type = String)]
        date: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    #[schema(example = "AAPL")]
    pub symbol: String,
    #[schema(example = 10)]
    pub quantity: u64,
    pub order_side: OrderSide,
    pub order_type: OrderType,
    /// Price is required for limit orders, ignored for market orders
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderResponse {
    #[schema(value_type = String, format = Uuid)]
    pub id: Uuid,
    #[schema(value_type = String, format = Uuid)]
    pub client_id: Uuid,
    #[schema(value_type = String)]
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrderRequest {
    pub quantity: Option<u64>,
    pub order_type: Option<OrderType>,
}

// ============================================================================
// Market Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketPriceResponse {
    #[schema(example = "AAPL")]
    pub symbol: String,
    #[schema(example = 175.50)]
    pub price: f64,
    #[schema(example = 174.20)]
    pub open: f64,
    #[schema(example = 176.80)]
    pub high: f64,
    #[schema(example = 173.90)]
    pub low: f64,
    #[schema(example = 1000000)]
    pub volume: u64,
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketPricesResponse {
    pub prices: Vec<MarketPriceResponse>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SymbolsResponse {
    pub symbols: Vec<String>,
    pub count: usize,
}

// ============================================================================
// Order Execution Models (for Market Data Service communication)
// ============================================================================

/// Request to market data service for order execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteOrderRequest {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: u64,
}

/// Response from market data service after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteOrderResponse {
    pub order_id: Uuid,
    pub status: ExecutionStatus,
    pub filled_quantity: u64,
    pub average_price: Option<f64>,
    pub message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Execution status from market data service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    FullyFilled,
    PartiallyFilled,
    Rejected,
}

// ============================================================================
// Internal service communication DTOs
// These mirror the service models but are independent
// ============================================================================

/// Response from user-service authentication endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AuthenticateResponse {
    pub authenticated: bool,
    pub message: String,
    pub user_id: Option<Uuid>,
}

/// Internal user model for deserializing from user-service responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UserServiceResponse {
    pub id: Option<Uuid>,
    pub email: String,
    pub password_hash: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub holdings: std::collections::HashMap<String, HoldingServiceResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HoldingServiceResponse {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
}

/// Internal order model for deserializing from order-service responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OrderServiceResponse {
    pub id: Uuid,
    pub client_id: Uuid,
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderStatusInternal,
    pub order_type: OrderTypeInternal,
    pub order_side: OrderSideInternal,
    pub filled_price: Option<f64>,
}

/// Paginated response from order service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OrderServicePaginatedResponse {
    pub data: Vec<OrderServiceResponse>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OrderSideInternal {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum OrderTypeInternal {
    Market,
    Limit { price: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum OrderStatusInternal {
    Pending,
    PartiallyFilled { filled_quantity: u64 },
    Filled { date: DateTime<Utc> },
    Rejected { date: DateTime<Utc> },
    Cancelled { date: DateTime<Utc> },
}

// ============================================================================
// Conversion utilities from internal service models to API models
// ============================================================================

impl From<UserServiceResponse> for UserResponse {
    fn from(user: UserServiceResponse) -> Self {
        Self {
            id: user.id.expect("User should have an ID"),
            email: user.email,
            firstname: user.firstname,
            surname: user.surname,
            balance: user.balance,
            is_verified: user.is_verified,
            created_at: user.created_at,
            holdings: user
                .holdings
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}

impl From<HoldingServiceResponse> for HoldingResponse {
    fn from(holding: HoldingServiceResponse) -> Self {
        Self {
            symbol: holding.symbol,
            quantity: holding.quantity,
            average_cost: holding.average_cost,
        }
    }
}

impl From<OrderSideInternal> for OrderSide {
    fn from(side: OrderSideInternal) -> Self {
        match side {
            OrderSideInternal::Buy => OrderSide::Buy,
            OrderSideInternal::Sell => OrderSide::Sell,
        }
    }
}

impl From<OrderSide> for OrderSideInternal {
    fn from(side: OrderSide) -> Self {
        match side {
            OrderSide::Buy => OrderSideInternal::Buy,
            OrderSide::Sell => OrderSideInternal::Sell,
        }
    }
}

impl From<OrderTypeInternal> for OrderType {
    fn from(order_type: OrderTypeInternal) -> Self {
        match order_type {
            OrderTypeInternal::Market => OrderType::Market,
            OrderTypeInternal::Limit { price } => OrderType::Limit { price },
        }
    }
}

impl From<OrderType> for OrderTypeInternal {
    fn from(order_type: OrderType) -> Self {
        match order_type {
            OrderType::Market => OrderTypeInternal::Market,
            OrderType::Limit { price } => OrderTypeInternal::Limit { price },
        }
    }
}

impl From<OrderStatusInternal> for OrderStatus {
    fn from(status: OrderStatusInternal) -> Self {
        match status {
            OrderStatusInternal::Pending => OrderStatus::Pending,
            OrderStatusInternal::PartiallyFilled { filled_quantity } => {
                OrderStatus::PartiallyFilled { filled_quantity }
            }
            OrderStatusInternal::Filled { date } => OrderStatus::Filled { date },
            OrderStatusInternal::Rejected { date } => OrderStatus::Rejected { date },
            OrderStatusInternal::Cancelled { date } => OrderStatus::Cancelled { date },
        }
    }
}

impl OrderServiceResponse {
    pub fn into_response(self) -> OrderResponse {
        OrderResponse {
            id: self.id,
            client_id: self.client_id,
            date: self.date,
            symbol: self.symbol,
            quantity: self.quantity,
            status: self.status.into(),
            order_type: self.order_type.into(),
            order_side: self.order_side.into(),
        }
    }
}
