/// Configuration for the Market Data Service
/// Follows the 12-factor app principles by loading configuration from environment variables

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Config {
    #[must_use]
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("MARKET_DATA_SERVICE_PORT")
            .unwrap_or_else(|_| "3003".to_string())
            .parse()
            .expect("MARKET_DATA_SERVICE_PORT must be a valid port number");

        Self { port }
    }
}
