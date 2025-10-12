use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub user_service_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("ORDER_SERVICE_PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse()
            .expect("ORDER_SERVICE_PORT must be a valid port number");

        let host = std::env::var("ORDER_SERVICE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let user_service_url = std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        Self {
            port,
            host,
            user_service_url,
        }
    }
}
