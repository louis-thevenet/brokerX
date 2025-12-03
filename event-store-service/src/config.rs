use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        // Parse CORS allowed origins from comma-separated list
        let cors_origins = env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| String::new());

        let cors_allowed_origins: Vec<String> = cors_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let port = env::var("EVENT_STORE_SERVICE_PORT")
            .unwrap_or_else(|_| "3005".to_string())
            .parse()
            .expect("EVENT_STORE_SERVICE_PORT must be a valid port number");

        Self {
            port,
            cors_allowed_origins,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub redis_url: String,
}

impl DatabaseConfig {
    pub fn from_env() -> Self {
        DatabaseConfig {
            database_url: env::var("EVENT_STORE_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .unwrap_or_else(|_| {
                    "postgres://brokerx_user:password123@localhost:5435/brokerx_event_store"
                        .to_string()
                }),
            max_connections: env::var("EVENT_STORE_DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .unwrap_or(600),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6380".to_string()),
        }
    }
}
