use crate::error::{ApiError, ApiResult};
use crate::models::{
    AuthenticateResponse, CreateOrderRequest, ExecuteOrderRequest, ExecuteOrderResponse,
    OrderResponse, OrderServicePaginatedResponse, OrderServiceResponse, PaginatedResponse,
    PaginationQuery, RegisterRequest, UpdateOrderRequest, UpdateUserRequest, UserResponse,
    UserServiceResponse,
};
use reqwest::Client;
use serde_json::json;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct UserClient {
    base_url: String,
    client: Client,
}

impl UserClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn create_user(&self, request: &RegisterRequest) -> ApiResult<Uuid> {
        debug!("Creating user: {:?}", request);
        let response = self
            .client
            .post(format!("{}/users", self.base_url))
            .json(&json!({
                "email": request.email,
                "password": request.password,
                "firstname": request.firstname,
                "surname": request.surname,
                "initial_balance": 0
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let user_id: String = response.json().await?;
            Uuid::parse_str(&user_id)
                .map_err(|_| ApiError::InternalServerError("Invalid user ID format".to_string()))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("User could not be created: {error_text}");
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn authenticate_user(&self, email: &str, password: &str) -> ApiResult<Uuid> {
        debug!("Authenticating user with email: {}", email);
        let response = self
            .client
            .post(format!("{}/auth/login", self.base_url))
            .json(&json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let auth_response: AuthenticateResponse = response.json().await?;
            if auth_response.authenticated {
                auth_response.user_id.ok_or(ApiError::InternalServerError(
                    "User ID missing in authentication response".to_string(),
                ))
            } else {
                error!("Authentication failed: {}", auth_response.message);
                Err(ApiError::Unauthorized)
            }
        } else {
            let error_text = response.text().await.unwrap_or_default();
            error!("Authentication failed: {error_text}");
            Err(ApiError::Unauthorized)
        }
    }

    pub async fn get_user(&self, user_id: Uuid) -> ApiResult<UserResponse> {
        let response = self
            .client
            .get(format!("{}/users/{}", self.base_url, user_id))
            .send()
            .await?;

        if response.status().is_success() {
            let user: UserServiceResponse = response.json().await?;
            Ok(user.into())
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

    pub async fn update_user(
        &self,
        user_id: Uuid,
        request: &UpdateUserRequest,
    ) -> ApiResult<UserResponse> {
        let response = self
            .client
            .put(format!("{}/users/{}", self.base_url, user_id))
            .json(request)
            .send()
            .await?;

        if response.status().is_success() {
            let user: UserServiceResponse = response.json().await?;
            Ok(user.into())
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

    pub async fn verify_email(&self, user_id: Uuid) -> ApiResult<()> {
        let response = self
            .client
            .post(format!("{}/users/{}/verify", self.base_url, user_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn deposit_funds(&self, user_id: Uuid, amount: f64) -> ApiResult<UserResponse> {
        debug!("Depositing {} to user ID {}", amount, user_id);
        let response = self
            .client
            .post(format!("{}/users/{}/deposit", self.base_url, user_id))
            .json(&json!({ "amount": amount }))
            .send()
            .await?;

        if response.status().is_success() {
            let user: UserServiceResponse = response.json().await?;
            info!("Deposited funds. New balance: {}", user.balance);
            Ok(user.into())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("User not found for deposit");
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Error depositing funds: {}", error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    pub async fn get_user_id_by_email(&self, email: &str) -> ApiResult<Uuid> {
        debug!("Fetching user ID for email: {}", email);
        let response = self
            .client
            .get(format!("{}/users/by-email/{}", self.base_url, email))
            .send()
            .await?;

        if response.status().is_success() {
            let user: UserServiceResponse = response.json().await?;
            info!("Fetched user: {}", user.email);
            user.id
                .ok_or(ApiError::InternalServerError("User ID missing".to_string()))
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("User not found");
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Error fetching user ID by email: {}", error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {status}: {error_text}"
            )))
        }
    }

    /// Update user holdings after order execution
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
            .post(format!("{}/users/{}/holdings", self.base_url, user_id))
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
                "User service error {}: {}",
                status, error_text
            )))
        }
    }
}

pub struct OrderClient {
    base_url: String,
    client: Client,
}

impl OrderClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
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
            .post(format!("{}/orders", self.base_url))
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

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> ApiResult<OrderResponse> {
        let response = self
            .client
            .get(format!("{}/orders/{}", self.base_url, order_id))
            .send()
            .await?;

        if response.status().is_success() {
            let order: OrderServiceResponse = response.json().await?;
            // Verify the order belongs to the requesting user
            if order.client_id == user_id {
                Ok(order.into_response())
            } else {
                Err(ApiError::Forbidden)
            }
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {status}: {error_text}"
            )))
        }
    }

    pub async fn get_user_orders(
        &self,
        user_id: Uuid,
        pagination: &PaginationQuery,
    ) -> ApiResult<PaginatedResponse<OrderResponse>> {
        let query_params = vec![
            ("client_id", user_id.to_string()),
            ("page", pagination.page.to_string()),
            ("limit", pagination.limit.to_string()),
        ];

        let response = self
            .client
            .get(&format!("{}/orders", self.base_url))
            .query(&query_params)
            .send()
            .await?;

        if response.status().is_success() {
            // The order service now returns proper paginated response
            let paginated: OrderServicePaginatedResponse = response.json().await?;

            let order_responses: Vec<OrderResponse> = paginated
                .data
                .into_iter()
                .map(|o| o.into_response())
                .collect();

            Ok(PaginatedResponse {
                data: order_responses,
                pagination: paginated.pagination,
            })
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn update_order(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        request: &UpdateOrderRequest,
    ) -> ApiResult<OrderResponse> {
        let response = self
            .client
            .put(&format!("{}/orders/{}", self.base_url, order_id))
            .json(request)
            .send()
            .await?;

        if response.status().is_success() {
            let order: OrderServiceResponse = response.json().await?;
            // Verify ownership
            if order.client_id == user_id {
                Ok(order.into_response())
            } else {
                Err(ApiError::Forbidden)
            }
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else if response.status() == reqwest::StatusCode::FORBIDDEN {
            Err(ApiError::Forbidden)
        } else if response.status() == reqwest::StatusCode::CONFLICT {
            Err(ApiError::BadRequest("Order cannot be modified".to_string()))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn cancel_order(&self, _user_id: Uuid, order_id: Uuid) -> ApiResult<()> {
        let response = self
            .client
            .delete(&format!("{}/orders/{}", self.base_url, order_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else if response.status() == reqwest::StatusCode::FORBIDDEN {
            Err(ApiError::Forbidden)
        } else if response.status() == reqwest::StatusCode::CONFLICT {
            Err(ApiError::BadRequest(
                "Order cannot be cancelled".to_string(),
            ))
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    /// Update order with execution results from market data service
    pub async fn update_order_execution(
        &self,
        order_id: Uuid,
        execution_status: crate::models::ExecutionStatus,
        filled_quantity: u64,
        filled_price: Option<f64>,
    ) -> ApiResult<OrderResponse> {
        debug!(
            "Updating order {} execution: status={:?}, filled={}, price={:?}",
            order_id, execution_status, filled_quantity, filled_price
        );

        // Map execution status to order status
        let order_status = match execution_status {
            crate::models::ExecutionStatus::FullyFilled => {
                json!({"status": "filled", "date": chrono::Utc::now()})
            }
            crate::models::ExecutionStatus::PartiallyFilled => {
                json!({"status": "partially_filled", "filled_quantity": filled_quantity})
            }
            crate::models::ExecutionStatus::Rejected => {
                json!({"status": "rejected", "date": chrono::Utc::now()})
            }
        };

        let response = self
            .client
            .patch(format!("{}/orders/{}/execution", self.base_url, order_id))
            .json(&json!({
                "status": order_status,
                "filled_price": filled_price
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let order: OrderServiceResponse = response.json().await?;
            info!("Order {} execution updated successfully", order_id);
            Ok(order.into_response())
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("Order {} not found for execution update", order_id);
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Error updating order execution: {}", error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }
}

// ============================================================================
// Market Data Client
// ============================================================================

pub struct MarketDataClient {
    base_url: String,
    client: Client,
}

impl MarketDataClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    /// Get current price for a specific symbol
    pub async fn get_price(&self, symbol: &str) -> ApiResult<serde_json::Value> {
        debug!("Fetching price for symbol: {}", symbol);
        let response = self
            .client
            .get(format!("{}/api/v1/prices/{}", self.base_url, symbol))
            .send()
            .await?;

        if response.status().is_success() {
            info!("Price data retrieved successfully for symbol: {}", symbol);
            let price = response.json().await?;
            Ok(price)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            error!("Symbol not found: {}", symbol);
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Error fetching price for symbol {}: {}", symbol, error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {status}: {error_text}"
            )))
        }
    }

    /// Get current prices for multiple symbols
    pub async fn get_prices(&self, symbols: &[String]) -> ApiResult<serde_json::Value> {
        debug!("Fetching prices for symbols: {:?}", symbols);
        let symbols_param = symbols.join(",");
        let response = self
            .client
            .get(format!("{}/api/v1/prices", self.base_url))
            .query(&[("symbols", symbols_param)])
            .send()
            .await?;

        if response.status().is_success() {
            let prices = response.json().await?;
            Ok(prices)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {}: {}",
                status, error_text
            )))
        }
    }

    /// Get all current prices (no symbol filter)
    pub async fn get_all_prices(&self) -> ApiResult<serde_json::Value> {
        debug!("Fetching all available prices");
        let response = self
            .client
            .get(format!("{}/api/v1/prices", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            let prices = response.json().await?;
            Ok(prices)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {}: {}",
                status, error_text
            )))
        }
    }

    /// Get list of available symbols
    pub async fn get_symbols(&self) -> ApiResult<serde_json::Value> {
        debug!("Fetching available symbols");
        let response = self
            .client
            .get(format!("{}/api/v1/symbols", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            let symbols = response.json().await?;
            Ok(symbols)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {}: {}",
                status, error_text
            )))
        }
    }

    /// Get historical prices for a symbol
    pub async fn get_historical_prices(
        &self,
        symbol: &str,
        limit: Option<i64>,
    ) -> ApiResult<serde_json::Value> {
        debug!("Fetching historical prices for symbol: {}", symbol);
        let mut req = self.client.get(format!(
            "{}/api/v1/prices/{}/history",
            self.base_url, symbol
        ));

        if let Some(limit) = limit {
            req = req.query(&[("limit", limit.to_string())]);
        }

        let response = req.send().await?;

        if response.status().is_success() {
            let history = response.json().await?;
            Ok(history)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {}: {}",
                status, error_text
            )))
        }
    }
}

// ============================================================================
// Market Data Client (for Order Execution)
// ============================================================================

pub struct MarketDataExecutionClient {
    base_url: String,
    client: Client,
}

impl MarketDataExecutionClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    /// Execute an order through the market data service
    pub async fn execute_order(
        &self,
        request: crate::models::ExecuteOrderRequest,
    ) -> ApiResult<crate::models::ExecuteOrderResponse> {
        info!(
            "Executing order {} for {:?} {} shares of {}",
            request.order_id, request.side, request.quantity, request.symbol
        );

        let response = self
            .client
            .post(format!("{}/api/v1/execute", self.base_url))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let execution: crate::models::ExecuteOrderResponse = response.json().await?;
            info!(
                "Order {} execution result: {:?}, filled: {}, price: {:?}",
                execution.order_id, execution.status, execution.filled_quantity, execution.average_price
            );
            Ok(execution)
        } else if response.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
            // Order was rejected
            let execution: crate::models::ExecuteOrderResponse = response.json().await?;
            info!("Order {} rejected: {:?}", execution.order_id, execution.message);
            Ok(execution)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Market data execution error {}: {}", status, error_text);
            Err(ApiError::ServiceUnavailable(format!(
                "Market data service error {}: {}",
                status, error_text
            )))
        }
    }
}
