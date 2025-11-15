use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Type Aliases
// ============================================================================

pub type EventId = Uuid;
pub type OrderId = Uuid;
pub type UserId = Uuid;
pub type CorrelationId = Uuid;

// ============================================================================
// Order Event Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderEventType {
    /// Order has been created by the API Gateway
    OrderCreated,
    /// Order has passed pre-trade validation
    OrderValidated,
    /// Order was rejected during validation
    OrderRejected,
    /// Order has been sent to market for execution
    OrderExecuted,
    /// Order has been completely filled
    OrderFilled,
    /// Order has been partially filled
    OrderPartiallyFilled,
    /// Funds have been debited from user account (buy orders)
    FundsDebited,
    /// Funds have been credited to user account (sell orders)
    FundsCredited,
    /// User holdings have been updated
    HoldingsUpdated,
    /// Order saga has completed successfully
    OrderCompleted,
}

impl std::fmt::Display for OrderEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderEventType::OrderCreated => write!(f, "ORDER_CREATED"),
            OrderEventType::OrderValidated => write!(f, "ORDER_VALIDATED"),
            OrderEventType::OrderRejected => write!(f, "ORDER_REJECTED"),
            OrderEventType::OrderExecuted => write!(f, "ORDER_EXECUTED"),
            OrderEventType::OrderFilled => write!(f, "ORDER_FILLED"),
            OrderEventType::OrderPartiallyFilled => write!(f, "ORDER_PARTIALLY_FILLED"),
            OrderEventType::FundsDebited => write!(f, "FUNDS_DEBITED"),
            OrderEventType::FundsCredited => write!(f, "FUNDS_CREDITED"),
            OrderEventType::HoldingsUpdated => write!(f, "HOLDINGS_UPDATED"),
            OrderEventType::OrderCompleted => write!(f, "ORDER_COMPLETED"),
        }
    }
}

// ============================================================================
// Order Side & Type (matching order-service models)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "buy"),
            OrderSide::Sell => write!(f, "sell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit { price: f64 },
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "market"),
            OrderType::Limit { .. } => write!(f, "limit"),
        }
    }
}

// ============================================================================
// Saga Steps
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SagaStep {
    /// Order created, waiting for validation
    Created,
    /// Validation in progress
    Validating,
    /// Validation successful
    Validated,
    /// Execution in progress
    Executing,
    /// Execution completed
    Executed,
    /// Settlement (funds & holdings update) in progress
    Settling,
    /// Saga completed successfully
    Completed,
    /// Saga failed
    Failed,
    /// Compensation in progress
    Compensating,
    /// Compensation completed
    Compensated,
}

impl std::fmt::Display for SagaStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SagaStep::Created => write!(f, "CREATED"),
            SagaStep::Validating => write!(f, "VALIDATING"),
            SagaStep::Validated => write!(f, "VALIDATED"),
            SagaStep::Executing => write!(f, "EXECUTING"),
            SagaStep::Executed => write!(f, "EXECUTED"),
            SagaStep::Settling => write!(f, "SETTLING"),
            SagaStep::Completed => write!(f, "COMPLETED"),
            SagaStep::Failed => write!(f, "FAILED"),
            SagaStep::Compensating => write!(f, "COMPENSATING"),
            SagaStep::Compensated => write!(f, "COMPENSATED"),
        }
    }
}

// ============================================================================
// Core Domain Models
// ============================================================================

/// Represents a single event in the order processing saga
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderEvent {
    #[schema(value_type = String, format = Uuid)]
    pub event_id: EventId,

    #[schema(value_type = String, format = Uuid)]
    pub order_id: OrderId,

    #[schema(value_type = String, format = Uuid)]
    pub user_id: UserId,

    pub event_type: OrderEventType,

    // Order details
    #[schema(example = "AAPL")]
    pub symbol: String,

    #[schema(example = 100)]
    pub quantity: i64,

    pub order_side: OrderSide,
    pub order_type: OrderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.50)]
    pub price: Option<f64>,

    // Execution results
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 100)]
    pub filled_quantity: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.75)]
    pub average_price: Option<f64>,

    // Event metadata
    #[schema(value_type = String)]
    pub occurred_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<i64>,

    // Saga correlation
    #[schema(value_type = String, format = Uuid)]
    pub correlation_id: CorrelationId,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Uuid)]
    pub causation_id: Option<EventId>,

    // Additional data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// State of an order saga
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SagaState {
    #[schema(value_type = String, format = Uuid)]
    pub order_id: OrderId,

    #[schema(value_type = String, format = Uuid)]
    pub user_id: UserId,

    pub current_step: SagaStep,

    // Order details
    #[schema(example = "AAPL")]
    pub symbol: String,

    #[schema(example = 100)]
    pub quantity: i64,

    pub order_side: OrderSide,
    pub order_type: OrderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.50)]
    pub price: Option<f64>,

    // Tracking
    #[schema(value_type = String)]
    pub started_at: DateTime<Utc>,

    #[schema(value_type = String)]
    pub updated_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub completed_at: Option<DateTime<Utc>>,

    // Results
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 100)]
    pub filled_quantity: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.75)]
    pub average_price: Option<f64>,

    // Error handling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    pub compensation_needed: bool,

    // Correlation
    #[schema(value_type = String, format = Uuid)]
    pub correlation_id: CorrelationId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Message in the outbox table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessage {
    pub outbox_id: Uuid,
    pub order_id: OrderId,
    pub event_type: OrderEventType,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub processing_attempts: i32,
    pub last_error: Option<String>,
}

/// Subscription checkpoint for event stream consumers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub subscription_id: String,
    pub last_processed_sequence: i64,
    pub last_processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

/// Request to publish a new event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishEventRequest {
    #[schema(value_type = String, format = Uuid)]
    pub order_id: OrderId,

    #[schema(value_type = String, format = Uuid)]
    pub user_id: UserId,

    pub event_type: OrderEventType,

    #[schema(example = "AAPL")]
    pub symbol: String,

    #[schema(example = 100)]
    pub quantity: i64,

    pub order_side: OrderSide,
    pub order_type: OrderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.50)]
    pub price: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 100)]
    pub filled_quantity: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 150.75)]
    pub average_price: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Uuid)]
    pub correlation_id: Option<CorrelationId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, format = Uuid)]
    pub causation_id: Option<EventId>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response after publishing an event
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishEventResponse {
    #[schema(value_type = String, format = Uuid)]
    pub event_id: EventId,

    pub event_type: OrderEventType,

    #[schema(example = "Event published successfully")]
    pub message: String,

    #[schema(example = 12345)]
    pub sequence_number: i64,
}

/// Query parameters for event history
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EventHistoryQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
