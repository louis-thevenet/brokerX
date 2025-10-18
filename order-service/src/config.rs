use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("ORDER_SERVICE_PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse()
            .expect("ORDER_SERVICE_PORT must be a valid port number");

        Self { port }
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
