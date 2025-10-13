/// Market Data API Handlers
use crate::models::{
    HistoricalPricesQuery, HistoricalPricesResponse, MarketPriceResponse, MarketPricesResponse,
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
}

// ============================================================================
// Response Models
// ============================================================================

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

// ============================================================================
// Error Handling
// ============================================================================

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
            _ => ApiError::InternalError(err.to_string()),
        }
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Health check endpoint
/// Returns the service status and version information
#[utoipa::path(
    get,
    path = "/health",
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
    path = "/api/v1/prices/{symbol}",
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
    use tracing::info;
    info!("Fetching current price for symbol: {}", symbol);
    let price = state.market_data_service.get_current_price(&symbol).await?;
    info!("Successfully retrieved price for {}: ${}", symbol, price.price);
    Ok(Json(price))
}

/// Get current prices for multiple symbols
/// Query parameter: symbols=AAPL,GOOGL,MSFT
#[utoipa::path(
    get,
    path = "/api/v1/prices",
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
    use tracing::info;
    if let Some(symbols_str) = params.get("symbols") {
        let symbols: Vec<String> = symbols_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        info!("Fetching current prices for {} symbols: {:?}", symbols.len(), symbols);
        let prices = state
            .market_data_service
            .get_current_prices(symbols)
            .await?;
        info!("Successfully retrieved {} prices", prices.count);
        Ok(Json(prices))
    } else {
        // If no symbols specified, return all available prices
        info!("Fetching all current prices");
        let prices = state.market_data_service.get_all_current_prices().await?;
        info!("Successfully retrieved all {} prices", prices.count);
        Ok(Json(prices))
    }
}

/// Get historical prices for a symbol
/// Supports optional date range filtering and result limiting
#[utoipa::path(
    get,
    path = "/api/v1/prices/{symbol}/history",
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
    use tracing::info;
    info!(
        "Fetching historical prices for symbol: {} (limit: {})",
        symbol, query.limit
    );
    let prices = state
        .market_data_service
        .get_historical_prices(&symbol, query.start_date, query.end_date, query.limit)
        .await?;
    info!("Successfully retrieved {} historical prices for {}", prices.count, symbol);
    Ok(Json(prices))
}

/// Get list of all available symbols
/// Returns all symbols that have market data available
#[utoipa::path(
    get,
    path = "/api/v1/symbols",
    responses(
        (status = 200, description = "List of available symbols", body = SymbolsResponse)
    ),
    tag = "symbols"
)]
pub async fn get_available_symbols(
    State(state): State<AppState>,
) -> Result<Json<SymbolsResponse>, ApiError> {
    use tracing::info;
    info!("Fetching available symbols");
    let symbols = state.market_data_service.get_available_symbols().await?;
    let count = symbols.len();
    info!("Successfully retrieved {} available symbols", count);
    Ok(Json(SymbolsResponse { symbols, count }))
}

/// Initialize market data with predefined symbols
/// Creates initial price data for common symbols if they don't exist
#[utoipa::path(
    post,
    path = "/api/v1/initialize",
    responses(
        (status = 200, description = "Market data initialized", body = InitializeResponse)
    ),
    tag = "admin"
)]
#[axum::debug_handler]
pub async fn initialize_market_data(State(state): State<AppState>) -> impl IntoResponse {
    use tracing::info;
    info!("Initializing market data with predefined symbols");
    match state.market_data_service.initialize_market_data().await {
        Ok(symbols_created) => {
            info!("Successfully initialized {} symbols", symbols_created);
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
            info!("Failed to initialize market data: {}", e);
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
    path = "/api/v1/simulate",
    responses(
        (status = 200, description = "Prices updated", body = MessageResponse)
    ),
    tag = "admin"
)]
#[axum::debug_handler]
pub async fn simulate_price_update(State(state): State<AppState>) -> impl IntoResponse {
    use tracing::info;
    info!("Starting price simulation for all symbols");
    match state.market_data_service.simulate_price_update().await {
        Ok(updated_count) => {
            info!("Successfully simulated price updates for {} symbols", updated_count);
            (
                StatusCode::OK,
                Json(MessageResponse {
                    message: format!("Updated {updated_count} symbol prices"),
                }),
            )
                .into_response()
        }
        Err(e) => {
            info!("Failed to simulate price updates: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
