/// Configuration for the Market Data Service
/// Follows the 12-factor app principles by loading configuration from environment variables

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    #[must_use]
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        // Parse CORS allowed origins from comma-separated list
        let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| String::new());

        let cors_allowed_origins: Vec<String> = cors_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let port = std::env::var("MARKET_DATA_SERVICE_PORT")
            .unwrap_or_else(|_| "3003".to_string())
            .parse()
            .expect("MARKET_DATA_SERVICE_PORT must be a valid port number");

        Self {
            port,
            cors_allowed_origins,
        }
    }
}
