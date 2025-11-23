use std::env;

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
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6380".to_string()),
        }
    }
}
