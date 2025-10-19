#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("USER_SERVICE_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .expect("USER_SERVICE_PORT must be a valid port number");

        Self { port }
    }
}
