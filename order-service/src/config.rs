#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        // Parse CORS allowed origins from comma-separated list
        let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| String::new());

        let cors_allowed_origins: Vec<String> = cors_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let port = std::env::var("ORDER_SERVICE_PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse()
            .expect("ORDER_SERVICE_PORT must be a valid port number");

        Self {
            port,
            cors_allowed_origins,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
}

impl DatabaseConfig {
    /// Create a configuration from a specific URL
    pub fn from_url(database_url: String) -> Self {
        Self { database_url }
    }
}
