mod api;
mod config;
mod logging;
mod services;

use color_eyre::Result;
use domain::core::BrokerX;
use services::BrokerHandle;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize logging
    logging::init()?;
    tracing::info!("Starting BrokerX application");

    let broker_x = BrokerX::new().await;
    broker_x.debug_populate().await;
    broker_x.start_order_processing().await;
    tracing::debug!("BrokerX initialized: {broker_x:#?}");

    let app_state = BrokerHandle::new(broker_x);
    let app = api::create_api(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
