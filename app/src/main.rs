mod api;
mod config;
mod logging;

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
    broker_x.start_order_processing();
    tracing::debug!("BrokerX initialized: {broker_x:#?}");

    let app_state = Arc::new(Mutex::new(broker_x));
    let app = api::create_api(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
