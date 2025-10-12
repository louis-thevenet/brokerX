use crate::error::{ApiError, ApiResult};
use crate::models::*;
use reqwest::Client;
use serde_json::json;
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
        let response = self
            .client
            .post(&format!("{}/users", self.base_url))
            .json(&json!({
                "email": request.email,
                "password": request.password,
                "firstname": request.firstname,
                "surname": request.surname,
                "initial_balance": request.initial_balance
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
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn authenticate_user(&self, email: &str, password: &str) -> ApiResult<Uuid> {
        let response = self
            .client
            .post(&format!("{}/auth/login", self.base_url))
            .json(&json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        if response.status().is_success() {
            // Assuming the user service returns user info on successful auth
            let user: User = response.json().await?;
            user.id
                .ok_or(ApiError::InternalServerError("User ID missing".to_string()))
        } else {
            Err(ApiError::Unauthorized)
        }
    }

    pub async fn get_user(&self, user_id: Uuid) -> ApiResult<User> {
        let response = self
            .client
            .get(&format!("{}/users/{}", self.base_url, user_id))
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
                "User service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn update_user(&self, user_id: Uuid, request: &UpdateUserRequest) -> ApiResult<User> {
        let response = self
            .client
            .put(&format!("{}/users/{}", self.base_url, user_id))
            .json(request)
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
                "User service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn verify_email(&self, user_id: Uuid) -> ApiResult<()> {
        let response = self
            .client
            .post(&format!("{}/users/{}/verify", self.base_url, user_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "User service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn deposit_funds(&self, user_id: Uuid, amount: f64) -> ApiResult<User> {
        // This would depend on how the user service implements fund deposits
        // For now, assuming it's a simple balance update
        let response = self
            .client
            .post(&format!("{}/users/{}/deposit", self.base_url, user_id))
            .json(&json!({ "amount": amount }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
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
    ) -> ApiResult<Order> {
        let response = self
            .client
            .post(&format!("{}/orders", self.base_url))
            .json(&json!({
                "client_id": user_id,
                "symbol": request.symbol,
                "quantity": request.quantity,
                "order_side": request.order_side,
                "order_type": request.order_type,
                "price": request.price
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn get_order(&self, user_id: Uuid, order_id: Uuid) -> ApiResult<Order> {
        let response = self
            .client
            .get(&format!("{}/orders/{}", self.base_url, order_id))
            .send()
            .await?;

        if response.status().is_success() {
            let order: Order = response.json().await?;
            // Verify the order belongs to the requesting user
            if order.client_id == user_id {
                Ok(order)
            } else {
                Err(ApiError::Forbidden)
            }
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ApiError::NotFound)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn get_user_orders(
        &self,
        user_id: Uuid,
        pagination: &PaginationQuery,
    ) -> ApiResult<PaginatedResponse<Order>> {
        let mut query_params = vec![
            ("client_id", user_id.to_string()),
            ("page", pagination.page.to_string()),
            ("limit", pagination.limit.to_string()),
        ];

        if let Some(status) = &pagination.status {
            query_params.push(("status", format!("{:?}", status)));
        }

        if let Some(symbol) = &pagination.symbol {
            query_params.push(("symbol", symbol.clone()));
        }

        let response = self
            .client
            .get(&format!("{}/orders", self.base_url))
            .query(&query_params)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
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
    ) -> ApiResult<Order> {
        // First verify the order belongs to the user
        let existing_order = self.get_order(user_id, order_id).await?;

        if existing_order.client_id != user_id {
            return Err(ApiError::Forbidden);
        }

        let response = self
            .client
            .put(&format!("{}/orders/{}", self.base_url, order_id))
            .json(request)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }

    pub async fn cancel_order(&self, user_id: Uuid, order_id: Uuid) -> ApiResult<()> {
        // First verify the order belongs to the user
        let existing_order = self.get_order(user_id, order_id).await?;

        if existing_order.client_id != user_id {
            return Err(ApiError::Forbidden);
        }

        let response = self
            .client
            .delete(&format!("{}/orders/{}", self.base_url, order_id))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            Err(ApiError::ServiceUnavailable(format!(
                "Order service error {}: {}",
                status, error_text
            )))
        }
    }
}
