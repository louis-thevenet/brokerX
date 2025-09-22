mod config;
mod logging;
mod web;

use color_eyre::Result;
use domain::core::BrokerX;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize logging
    logging::init()?;
    tracing::info!("Starting BrokerX application");

    let mut broker_x = BrokerX::new();
    broker_x.debug_populate();
    tracing::debug!("BrokerX initialized: {broker_x:#?}");

    let app_state = Arc::new(Mutex::new(broker_x));
    let app = web::create_app(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
