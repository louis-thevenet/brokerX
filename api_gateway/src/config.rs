use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_address: String,
    pub user_service_url: String,
    pub order_service_url: String,
    pub market_data_service_url: String,
    pub orchestrator_service_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub cors_allowed_origins: Vec<String>,
    pub mfa_bypass: bool,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> Config {
        // Parse CORS allowed origins from comma-separated list
        let cors_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:8000,http://127.0.0.1:8000".to_string());

        let cors_allowed_origins: Vec<String> = cors_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Config {
            server_address: env::var("API_GATEWAY_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            user_service_url: env::var("USER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            order_service_url: env::var("ORDER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3002".to_string()),
            market_data_service_url: env::var("MARKET_DATA_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3003".to_string()),
            orchestrator_service_url: env::var("ORCHESTRATOR_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:3004".to_string()),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
                "your-super-secret-jwt-key-change-this-in-production".to_string()
            }),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            cors_allowed_origins,
            mfa_bypass: env::var("MFA_BYPASS")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                == "true",
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        }
    }
}
