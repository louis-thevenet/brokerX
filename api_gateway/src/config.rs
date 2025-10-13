use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_address: String,
    pub user_service_url: String,
    pub order_service_url: String,
    pub market_data_service_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            server_address: env::var("API_GATEWAY_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            user_service_url: env::var("USER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            order_service_url: env::var("ORDER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3002".to_string()),
            market_data_service_url: env::var("MARKET_DATA_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3003".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
                "your-super-secret-jwt-key-change-this-in-production".to_string()
            }),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
        })
    }
}
