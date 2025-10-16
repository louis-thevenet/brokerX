use std::env;

/// Generic database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
}

impl DatabaseConfig {
    /// Load database configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        Self { database_url }
    }

    /// Create a configuration from a specific URL
    pub fn from_url(database_url: String) -> Self {
        Self { database_url }
    }

    /// Create a configuration for testing
    pub fn for_testing(test_db_name: &str) -> Self {
        dotenvy::dotenv().ok();

        let base_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/brokerx".to_string());
        let database_url = base_url.replace("/brokerx", &format!("/brokerx_{test_db_name}_test"));

        Self { database_url }
    }
}
