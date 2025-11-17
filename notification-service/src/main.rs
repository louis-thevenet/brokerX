use color_eyre::Result;
use notification_service::{NotificationEventConsumer, NotificationService};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

#[derive(Debug)]
struct Config {
    redis_url: String,
    user_service_url: String,
    port: u16,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let _ = dotenvy::dotenv();

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let user_service_url = std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:3001".to_string());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3005".to_string())
            .parse()
            .map_err(|_| "Invalid PORT: must be a valid port number".to_string())?;

        Ok(Self {
            redis_url,
            user_service_url,
            port,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize color-eyre for better error handling
    color_eyre::install()?;

    info!("Starting BrokerX Notification Service...");

    // Load configuration
    let config = Config::from_env().map_err(|e| color_eyre::eyre::eyre!(e))?;

    info!(
        "Configuration loaded - Redis: {}, User Service: {}, Port: {}",
        config.redis_url, config.user_service_url, config.port
    );

    // Create notification service
    let notification_service = Arc::new(NotificationService::new());
    info!("Notification service initialized");

    // Start retry processor
    NotificationService::start_retry_processor(notification_service.clone());
    info!("Retry processor started");

    // Create and start event consumer
    let consumer = NotificationEventConsumer::new(
        notification_service.clone(),
        config.user_service_url.clone(),
        config.redis_url.clone(),
    );

    info!("Event consumer initialized, starting to listen for events...");

    // Start the event consumer in a separate task
    let consumer_handle = tokio::spawn(async move {
        if let Err(e) = consumer.start().await {
            error!("Event consumer error: {}", e);
        }
    });

    info!("Notification Service is ready and listening for events");
    info!("Email notifications: Enabled (check SMTP configuration)");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutdown signal received, stopping services...");

    // Cancel the consumer task
    consumer_handle.abort();

    info!("Notification Service stopped");
    Ok(())
}
