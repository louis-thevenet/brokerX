#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub user_service_url: String,
    pub order_service_url: String,
    pub market_data_service_url: String,
    pub api_gateway_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("ORDER_EXECUTION_ORCHESTRATOR_SERVICE_PORT")
            .unwrap_or_else(|_| "3004".to_string())
            .parse()
            .expect("ORDER_EXECUTION_ORCHESTRATOR_SERVICE_PORT must be a valid port number");

        let user_service_url = std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        let order_service_url = std::env::var("ORDER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3002".to_string());

        let market_data_service_url = std::env::var("MARKET_DATA_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3003".to_string());

        let api_gateway_url = std::env::var("API_GATEWAY_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        Self {
            port,
            user_service_url,
            order_service_url,
            market_data_service_url,
            api_gateway_url,
        }
    }
}
