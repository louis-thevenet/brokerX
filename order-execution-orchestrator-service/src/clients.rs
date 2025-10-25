use crate::error::{ApiError, ApiResult};
use crate::models::{CreateOrderRequest, OrderResponse, OrderSide, OrderType};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::{debug, error, info};
use uuid::Uuid;

// ============================================================================
// API Gateway Client (for cached prices)
// ============================================================================

pub struct ApiGatewayClient {
    base_url: String,
    client: Client,
}

impl ApiGatewayClient {
    pub fn new(base_url: &str) -> Self {
        // Configure HTTP client for high performance
        // Note: API Gateway has caching, but cache misses require upstream calls
        // so we need reasonable timeouts to allow cache population
        let client = Client::builder()
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_secs(2)) // Increased for cache misses
            .connect_timeout(std::time::Duration::from_millis(500)) // Increased for reliability
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    /// Get price from API Gateway (uses cached prices with 100ms TTL)
    pub async fn get_price(&self, symbol: &str) -> ApiResult<MarketPrice> {
        let url = format!("{}/v1/market/prices/{}", self.base_url, symbol);
        info!("📡 Fetching price for {} from API Gateway: {}", symbol, url);

        let response = match self.client.get(&url).send().await {
            Ok(resp) => {
                info!(
                    "✅ Got response from API Gateway for {}: status={}",
                    symbol,
                    resp.status()
                );
                resp
            }
            Err(e) => {
                error!("❌ Failed to connect to API Gateway at {}: {}", url, e);
                if e.is_timeout() {
                    error!("⏱️  TIMEOUT calling API Gateway for {}", symbol);
                } else if e.is_connect() {
                    error!("🔌 CONNECTION ERROR to API Gateway for {}", symbol);
                }
                return Err(ApiError::from(e));
            }
        };

        if response.status().is_success() {
            match response.json().await {
                Ok(price) => {
                    info!("✅ Successfully fetched price for {}: {:?}", symbol, price);
                    Ok(price)
                }
                Err(e) => {
                    error!("❌ Failed to parse price response for {}: {}", symbol, e);
                    Err(ApiError::from(e))
                }
            }
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("🔍 Symbol {} not found at API Gateway (404)", symbol);
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "⚠️  API Gateway error for {}: {} - {}",
                symbol, status, error_text
            );
            Err(ApiError::ServiceUnavailable(format!(
                "API Gateway error {status}: {error_text}"
            )))
        }
    }
}

// ============================================================================
// User Client
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserServiceResponse {
    pub id: Option<Uuid>,
    pub email: String,
    pub firstname: String,
    pub surname: String,
    pub balance: f64,
    pub is_verified: bool,
    pub holdings: HashMap<String, HoldingResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingResponse {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
}

pub struct UserClient {
    base_url: String,
    client: Client,
}

impl UserClient {
    pub fn new(base_url: &str) -> Self {
        // Configure HTTP client for high performance
        let client = Client::builder()
            .pool_max_idle_per_host(100) // Reuse connections
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_millis(500)) // Fast timeout
            .connect_timeout(std::time::Duration::from_millis(200))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    pub async fn get_user(&self, user_id: Uuid) -> ApiResult<UserServiceResponse> {
        let response = self
            .client
            .get(format!("{}/v1/users/{}", self.base_url, user_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn update_holdings(
        &self,
        user_id: Uuid,
        symbol: &str,
        quantity_change: i64,
        execution_price: f64,
    ) -> ApiResult<()> {
        debug!(
            "Updating holdings for user {}: {} {} @ {}",
            user_id, quantity_change, symbol, execution_price
        );

        let response = self
            .client
            .post(format!("{}/v1/users/{}/holdings", self.base_url, user_id))
            .json(&json!({
                "symbol": symbol,
                "quantity_change": quantity_change,
                "execution_price": execution_price
            }))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Holdings updated successfully for user {}", user_id);
            Ok(())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("User not found for holdings update");
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Error updating holdings: {}", error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn withdraw_funds(&self, user_id: Uuid, amount: f64) -> ApiResult<()> {
        debug!("Withdrawing ${:.2} from user {}", amount, user_id);
        let response = self
            .client
            .post(format!("{}/v1/users/{}/withdraw", self.base_url, user_id))
            .json(&json!({ "amount": amount }))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Withdrawal successful for user {}", user_id);
            Ok(())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else if response.status() == reqwest::StatusCode::BAD_REQUEST {
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::BadRequest(error_text))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn deposit_funds(&self, user_id: Uuid, amount: f64) -> ApiResult<()> {
        debug!("Depositing ${:.2} to user {}", amount, user_id);
        let response = self
            .client
            .post(format!("{}/v1/users/{}/deposit", self.base_url, user_id))
            .json(&json!({ "amount": amount }))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Deposit successful for user {}", user_id);
            Ok(())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }
}

// ============================================================================
// Order Client
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderServiceResponse {
    pub id: Uuid,
    pub client_id: Uuid,
    pub date: String,
    pub symbol: String,
    pub quantity: u64,
    pub status: OrderServiceStatus,
    pub order_type: OrderType,
    pub order_side: OrderSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OrderServiceStatus {
    Pending,
    PartiallyFilled { filled_quantity: u64 },
    Filled { date: String },
    Rejected { date: String },
    Cancelled { date: String },
}

impl OrderServiceResponse {
    pub fn into_response(self) -> OrderResponse {
        OrderResponse {
            id: self.id,
            client_id: self.client_id,
            date: chrono::DateTime::parse_from_rfc3339(&self.date)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc),
            symbol: self.symbol,
            quantity: self.quantity,
            status: self.status.into(),
            order_type: self.order_type,
            order_side: self.order_side,
        }
    }
}

impl From<OrderServiceStatus> for crate::models::OrderStatus {
    fn from(status: OrderServiceStatus) -> Self {
        match status {
            OrderServiceStatus::Pending => crate::models::OrderStatus::Pending,
            OrderServiceStatus::PartiallyFilled { filled_quantity } => {
                crate::models::OrderStatus::PartiallyFilled { filled_quantity }
            }
            OrderServiceStatus::Filled { date } => crate::models::OrderStatus::Filled {
                date: chrono::DateTime::parse_from_rfc3339(&date)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc),
            },
            OrderServiceStatus::Rejected { date } => crate::models::OrderStatus::Rejected {
                date: chrono::DateTime::parse_from_rfc3339(&date)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc),
            },
            OrderServiceStatus::Cancelled { date } => crate::models::OrderStatus::Cancelled {
                date: chrono::DateTime::parse_from_rfc3339(&date)
                    .unwrap_or_else(|_| chrono::Utc::now().into())
                    .with_timezone(&chrono::Utc),
            },
        }
    }
}

pub struct OrderClient {
    base_url: String,
    client: Client,
}

impl OrderClient {
    pub fn new(base_url: &str) -> Self {
        // Configure HTTP client for high performance
        let client = Client::builder()
            .pool_max_idle_per_host(100) // Reuse connections
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_millis(500)) // Fast timeout
            .connect_timeout(std::time::Duration::from_millis(200))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    pub async fn create_order(
        &self,
        user_id: Uuid,
        request: &CreateOrderRequest,
    ) -> ApiResult<OrderResponse> {
        debug!("Creating order for user ID {}: {:?}", user_id, request);
        let response = self
            .client
            .post(format!("{}/v1/orders", self.base_url))
            .json(&json!({
                "client_id": user_id,
                "symbol": request.symbol,
                "quantity": request.quantity,
                "order_side": request.order_side,
                "order_type": request.order_type,
            }))
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Order created successfully");
            let order: OrderServiceResponse = response.json().await?;
            Ok(order.into_response())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Order could not be created: {error_text}");
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {status}: {error_text}"
            )))
        }
    }

    pub async fn update_order_execution(
        &self,
        order_id: Uuid,
        execution_status: ExecutionStatus,
        filled_quantity: u64,
        filled_price: Option<f64>,
    ) -> ApiResult<OrderResponse> {
        debug!(
            "Updating order {} execution: {:?}, qty: {}, price: {:?}",
            order_id, execution_status, filled_quantity, filled_price
        );

        let status = match execution_status {
            ExecutionStatus::FullyFilled => json!({
                "status": "filled",
                "date": chrono::Utc::now().to_rfc3339()
            }),
            ExecutionStatus::PartiallyFilled => json!({
                "status": "partially_filled",
                "filled_quantity": filled_quantity
            }),
            ExecutionStatus::Rejected => json!({
                "status": "rejected",
                "date": chrono::Utc::now().to_rfc3339()
            }),
            ExecutionStatus::Pending => json!({
                "status": "pending"
            }),
        };

        let response = self
            .client
            .patch(format!(
                "{}/v1/orders/{}/execution",
                self.base_url, order_id
            ))
            .json(&json!({
                "status": status,
                "filled_price": filled_price
            }))
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Order execution updated successfully");
            let order: OrderServiceResponse = response.json().await?;
            Ok(order.into_response())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to update order execution: {error_text}");
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {status}: {error_text}"
            )))
        }
    }

    /// Get all pending orders (for recovery on startup)
    pub async fn get_pending_orders(&self) -> ApiResult<Vec<OrderResponse>> {
        debug!("Fetching all pending orders");
        let response = self
            .client
            .get(format!("{}/v1/orders/pending", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            let orders: Vec<OrderServiceResponse> = response.json().await?;
            info!("Fetched {} pending orders for recovery", orders.len());
            Ok(orders.into_iter().map(|o| o.into_response()).collect())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Failed to fetch pending orders: {}", error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {status}: {error_text}"
            )))
        }
    }
}

// ============================================================================
// Market Data Client
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteOrderRequest {
    pub order_id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    FullyFilled,
    PartiallyFilled,
    Rejected,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub order_id: Uuid,
    pub status: ExecutionStatus,
    pub filled_quantity: u64,
    pub average_price: Option<f64>,
    pub message: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPrice {
    pub symbol: String,
    pub price: f64,
    pub volume: u64,
    pub timestamp: String,
}

pub struct MarketDataClient {
    base_url: String,
    client: Client,
}

impl MarketDataClient {
    pub fn new(base_url: &str) -> Self {
        // Configure HTTP client for high performance
        let client = Client::builder()
            .pool_max_idle_per_host(100) // Reuse connections
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .timeout(std::time::Duration::from_millis(500)) // Fast timeout
            .connect_timeout(std::time::Duration::from_millis(200))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.to_string(),
            client,
        }
    }

    pub async fn execute_order(&self, request: ExecuteOrderRequest) -> ApiResult<ExecutionResult> {
        debug!("Executing order {:?}", request);
        let response = self
            .client
            .post(format!("{}/v1/execute", self.base_url))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let result: ExecutionResult = response.json().await?;
            info!("Order execution result: {:?}", result);
            Ok(result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Order execution failed: {error_text}");
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {status}: {error_text}"
            )))
        }
    }

    // Note: Price fetching is handled by ApiGatewayClient which implements caching
    // MarketDataClient is only used for order execution
}
