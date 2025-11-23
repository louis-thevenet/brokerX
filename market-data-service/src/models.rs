/// Market Data Models
/// These models represent the domain entities for market data and pricing information
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Represents a market price data point for an asset
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketPrice {
    /// Unique identifier for this price record
    pub id: Option<Uuid>,
    /// Asset symbol (e.g., AAPL, GOOGL, MSFT)
    pub symbol: String,
    /// Current price of the asset
    pub price: f64,
    /// Opening price for the current trading period
    pub open: f64,
    /// Highest price reached during the current trading period
    pub high: f64,
    /// Lowest price reached during the current trading period
    pub low: f64,
    /// Trading volume for the current period (i64 for PostgreSQL BIGINT compatibility)
    pub volume: i64,
    /// Timestamp when this price was recorded
    pub timestamp: DateTime<Utc>,
}

/// Response model for market price data (REST API)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketPriceResponse {
    /// Asset symbol
    #[schema(example = "AAPL")]
    pub symbol: String,
    /// Current price
    #[schema(example = 175.50)]
    pub price: f64,
    /// Opening price
    #[schema(example = 174.20)]
    pub open: f64,
    /// Highest price
    #[schema(example = 176.80)]
    pub high: f64,
    /// Lowest price
    #[schema(example = 173.90)]
    pub low: f64,
    /// Trading volume
    #[schema(example = 1000000)]
    pub volume: i64,
    /// Timestamp of the price data
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
}

/// Response model for multiple market prices
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketPricesResponse {
    pub prices: Vec<MarketPriceResponse>,
    pub count: usize,
}

/// Request to get historical prices for a symbol
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalPricesQuery {
    /// Start date for historical data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<DateTime<Utc>>,
    /// End date for historical data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<DateTime<Utc>>,
    /// Limit the number of results
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

/// Historical price data response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalPricesResponse {
    pub symbol: String,
    pub prices: Vec<MarketPriceResponse>,
    pub count: usize,
}

// Conversion from internal model to API response
impl From<MarketPrice> for MarketPriceResponse {
    fn from(price: MarketPrice) -> Self {
        Self {
            symbol: price.symbol,
            price: price.price,
            open: price.open,
            high: price.high,
            low: price.low,
            volume: price.volume,
            timestamp: price.timestamp,
        }
    }
}

/// Order execution request from API Gateway
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteOrderRequest {
    /// Order ID from order service
    #[schema(value_type = String)]
    pub order_id: Uuid,
    /// User ID who placed the order
    #[schema(value_type = String)]
    pub user_id: Uuid,
    /// Asset symbol
    #[schema(example = "AAPL")]
    pub symbol: String,
    /// Order side (buy or sell)
    pub side: OrderSide,
    /// Order type (market or limit)
    pub order_type: OrderType,
    /// Quantity of shares
    #[schema(example = 100)]
    pub quantity: u64,
}

/// Order side enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

/// Order execution response to API Gateway
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteOrderResponse {
    /// Order ID that was executed
    #[schema(value_type = String)]
    pub order_id: Uuid,
    /// Execution status
    pub status: ExecutionStatus,
    /// Number of shares filled
    #[schema(example = 100)]
    pub filled_quantity: u64,
    /// Average execution price (if filled)
    #[schema(example = 175.50)]
    pub average_price: Option<f64>,
    /// Additional information about execution
    pub message: Option<String>,
    /// Execution timestamp
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
}

/// Execution status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    FullyFilled,
    PartiallyFilled,
    Rejected,
}

/// Market simulation state for realistic execution
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketSimulationState {
    pub symbol: String,
    pub current_volatility: f64,
    pub trend_direction: f64,
    pub last_execution_price: Option<f64>,
    pub last_execution_time: Option<DateTime<Utc>>,
    pub daily_volume: i64,
    pub execution_count: i32,
}
