use crate::event_publisher::{EventPublisher, PublishEventRequest};
use crate::execution_simulator::ExecutionSimulator;
use crate::models::{
    ExecuteOrderRequest, ExecuteOrderResponse, HistoricalPricesQuery, HistoricalPricesResponse,
    MarketPriceResponse, MarketPricesResponse,
};
use crate::service::MarketDataService;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

/// Application state containing the market data service
#[derive(Clone)]
pub struct AppState {
    pub market_data_service: Arc<MarketDataService>,
    pub execution_simulator: Arc<ExecutionSimulator>,
    pub event_publisher: Arc<EventPublisher>,
}

// Response Models

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    #[schema(value_type = String)]
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SymbolsResponse {
    pub symbols: Vec<String>,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InitializeResponse {
    pub message: String,
    pub symbols_created: usize,
}

// Error Handling

pub enum ApiError {
    NotFound(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<crate::service::ServiceError> for ApiError {
    fn from(err: crate::service::ServiceError) -> Self {
        match err {
            crate::service::ServiceError::RepositoryError(
                crate::repository::RepositoryError::NotFound(symbol),
            ) => ApiError::NotFound(format!("Market data not found for symbol: {symbol}")),
            crate::service::ServiceError::RepositoryError(_) => {
                ApiError::InternalError(err.to_string())
            }
        }
    }
}

// API Handlers

/// Health check endpoint
/// Returns the service status and version information
#[utoipa::path(
    get,
    path = "/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "market-data-service".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Get current price for a specific symbol
/// Returns the latest market price data for the requested symbol
#[utoipa::path(
    get,
    path = "/v1/prices/{symbol}",
    params(
        ("symbol" = String, Path, description = "Asset symbol (e.g., AAPL, GOOGL)")
    ),
    responses(
        (status = 200, description = "Current price data", body = MarketPriceResponse),
        (status = 404, description = "Symbol not found")
    ),
    tag = "prices"
)]
pub async fn get_current_price(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> Result<Json<MarketPriceResponse>, ApiError> {
    let price = state.market_data_service.get_current_price(&symbol).await?;
    Ok(Json(price))
}

/// Get current prices for multiple symbols
/// Query parameter: symbols=AAPL,GOOGL,MSFT
#[utoipa::path(
    get,
    path = "/v1/prices",
    params(
        ("symbols" = Option<String>, Query, description = "Comma-separated list of symbols")
    ),
    responses(
        (status = 200, description = "Current prices for requested symbols", body = MarketPricesResponse)
    ),
    tag = "prices"
)]
pub async fn get_current_prices(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<MarketPricesResponse>, ApiError> {
    if let Some(symbols_str) = params.get("symbols") {
        let symbols: Vec<String> = symbols_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let prices = state
            .market_data_service
            .get_current_prices(symbols)
            .await?;
        Ok(Json(prices))
    } else {
        // If no symbols specified, return all available prices
        let prices = state.market_data_service.get_all_current_prices().await?;
        Ok(Json(prices))
    }
}

/// Get historical prices for a symbol
/// Supports optional date range filtering and result limiting
#[utoipa::path(
    get,
    path = "/v1/prices/{symbol}/history",
    params(
        ("symbol" = String, Path, description = "Asset symbol"),
        ("start_date" = Option<String>, Query, description = "Start date (ISO 8601)"),
        ("end_date" = Option<String>, Query, description = "End date (ISO 8601)"),
        ("limit" = Option<i64>, Query, description = "Maximum number of results (default: 100)")
    ),
    responses(
        (status = 200, description = "Historical price data", body = HistoricalPricesResponse),
        (status = 404, description = "Symbol not found")
    ),
    tag = "prices"
)]
pub async fn get_historical_prices(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<HistoricalPricesQuery>,
) -> Result<Json<HistoricalPricesResponse>, ApiError> {
    use tracing::{debug, info};
    info!("Getting historical prices for symbol: {}", symbol);
    debug!(
        "Query parameters: start={:?}, end={:?}, limit={}",
        query.start_date, query.end_date, query.limit
    );
    let prices = state
        .market_data_service
        .get_historical_prices(&symbol, query.start_date, query.end_date, query.limit)
        .await?;
    info!(
        "Historical prices retrieved successfully: {} records",
        prices.count
    );
    Ok(Json(prices))
}

/// Get list of all available symbols
/// Returns all symbols that have market data available
#[utoipa::path(
    get,
    path = "/v1/symbols",
    responses(
        (status = 200, description = "List of available symbols", body = SymbolsResponse)
    ),
    tag = "symbols"
)]
pub async fn get_available_symbols(
    State(state): State<AppState>,
) -> Result<Json<SymbolsResponse>, ApiError> {
    use tracing::info;
    info!("Getting available symbols");
    let symbols = state.market_data_service.get_available_symbols().await?;
    let count = symbols.len();
    info!(
        "Available symbols retrieved successfully: {} symbols",
        count
    );
    Ok(Json(SymbolsResponse { symbols, count }))
}

/// Initialize market data with predefined symbols
/// Creates initial price data for common symbols if they don't exist
#[utoipa::path(
    post,
    path = "/v1/initialize",
    responses(
        (status = 200, description = "Market data initialized", body = InitializeResponse)
    ),
    tag = "admin"
)]
#[axum::debug_handler]
pub async fn initialize_market_data(State(state): State<AppState>) -> impl IntoResponse {
    use tracing::{debug, info};
    info!("Initializing market data");
    match state.market_data_service.initialize_market_data().await {
        Ok(symbols_created) => {
            debug!("Created {} symbols", symbols_created);
            info!("Market data initialized successfully");
            (
                StatusCode::OK,
                Json(InitializeResponse {
                    message: "Market data initialized successfully".to_string(),
                    symbols_created,
                }),
            )
                .into_response()
        }
        Err(e) => {
            info!("Error initializing market data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// Simulate price update for all symbols
/// Generates new price data based on previous prices with realistic variations
/// This endpoint simulates market movement
#[utoipa::path(
    post,
    path = "/v1/simulate",
    responses(
        (status = 200, description = "Prices updated", body = MessageResponse)
    ),
    tag = "admin"
)]
#[axum::debug_handler]
pub async fn simulate_price_update(State(state): State<AppState>) -> impl IntoResponse {
    use tracing::{debug, info};
    info!("Simulating price updates");
    match state.market_data_service.simulate_price_update().await {
        Ok(updated_count) => {
            debug!("Updated {} symbols", updated_count);
            info!("Price simulation completed successfully");
            (
                StatusCode::OK,
                Json(MessageResponse {
                    message: format!("Updated {updated_count} symbol prices"),
                }),
            )
                .into_response()
        }
        Err(e) => {
            info!("Error simulating price updates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

/// Execute an order (called by API Gateway)
/// Simulates order execution and returns result
#[utoipa::path(
    post,
    path = "/v1/execute",
    request_body = ExecuteOrderRequest,
    responses(
        (status = 200, description = "Order executed successfully", body = ExecuteOrderResponse),
        (status = 422, description = "Order rejected", body = ExecuteOrderResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Symbol not found")
    ),
    tag = "execution"
)]
#[axum::debug_handler]
pub async fn execute_order(
    State(state): State<AppState>,
    Json(request): Json<ExecuteOrderRequest>,
) -> impl IntoResponse {
    use tracing::{debug, error, info};

    info!(
        "Executing order {} for {} {:?} {} shares",
        request.order_id, request.symbol, request.side, request.quantity
    );
    debug!(
        "Order details: type={:?}, side={:?}",
        request.order_type, request.side
    );

    match state
        .execution_simulator
        .execute_order(request.clone())
        .await
    {
        Ok(response) => {
            let status_code = match response.status {
                crate::models::ExecutionStatus::FullyFilled
                | crate::models::ExecutionStatus::PartiallyFilled => {
                    debug!(
                        "Order {} filled: {} shares @ ${:.2}",
                        response.order_id,
                        response.filled_quantity,
                        response.average_price.unwrap_or(0.0)
                    );
                    info!("Order {} executed successfully", response.order_id);

                    let event = PublishEventRequest {
                        order_id: response.order_id,
                        user_id: request.user_id,
                        event_type: "ORDER_EXECUTED".to_string(),
                        symbol: request.symbol.clone(),
                        quantity: request.quantity,
                        order_side: match request.side {
                            crate::models::OrderSide::Buy => brokerx_events::OrderSide::Buy,
                            crate::models::OrderSide::Sell => brokerx_events::OrderSide::Sell,
                        },
                        order_type: match request.order_type {
                            crate::models::OrderType::Market => brokerx_events::OrderType::Market,
                            crate::models::OrderType::Limit { price } => {
                                brokerx_events::OrderType::Limit { price }
                            }
                        },
                        filled_quantity: Some(response.filled_quantity),
                        average_price: response.average_price,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) = state.event_publisher.publish_event(event).await {
                        error!("Failed to publish ORDER_EXECUTED event: {}", e);
                    }

                    StatusCode::OK
                }
                crate::models::ExecutionStatus::Rejected => {
                    info!(
                        "Order {} rejected: {}",
                        response.order_id,
                        response.message.as_deref().unwrap_or("Unknown")
                    );

                    let event = PublishEventRequest {
                        order_id: response.order_id,
                        user_id: request.user_id,
                        event_type: "ORDER_REJECTED".to_string(),
                        symbol: request.symbol.clone(),
                        quantity: request.quantity,
                        order_side: match request.side {
                            crate::models::OrderSide::Buy => brokerx_events::OrderSide::Buy,
                            crate::models::OrderSide::Sell => brokerx_events::OrderSide::Sell,
                        },
                        order_type: match request.order_type {
                            crate::models::OrderType::Market => brokerx_events::OrderType::Market,
                            crate::models::OrderType::Limit { price } => {
                                brokerx_events::OrderType::Limit { price }
                            }
                        },
                        filled_quantity: None,
                        average_price: None,
                        occurred_at: chrono::Utc::now().to_rfc3339(),
                    };

                    if let Err(e) = state.event_publisher.publish_event(event).await {
                        error!("Failed to publish ORDER_REJECTED event: {}", e);
                    }

                    StatusCode::UNPROCESSABLE_ENTITY
                }
            };

            (status_code, Json(response)).into_response()
        }
        Err(e) => {
            error!("Order execution error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
