mod web;

use std::sync::{Arc, Mutex};
use color_eyre::Result;
use domain::core::BrokerX;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize the domain layer
    let mut broker_x = BrokerX::new();
    broker_x.debug_populate();
    println!("BrokerX initialized: {broker_x:#?}");

    // Wrap BrokerX in Arc<Mutex> for shared access
    let app_state = Arc::new(Mutex::new(broker_x));

    // Create the web application with state
    let app = web::create_app(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
