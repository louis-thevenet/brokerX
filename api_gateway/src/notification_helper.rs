use crate::error::{ApiError, ApiResult};
use crate::models::{HoldingResponse, MarketPriceResponse};
use notification_service::notification_models::{HoldingData, MarketDataSymbol, Notification};
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use tracing::{error, info};

pub struct NotificationHelper {
    redis_client: Client,
}

impl NotificationHelper {
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let redis_client = Client::open(redis_url)?;
        Ok(Self { redis_client })
    }

    /// Send immediate market data notification to a user
    pub async fn send_market_data_notification(
        &self,
        user_id: String,
        user_email: String,
        symbols_data: Vec<MarketPriceResponse>,
        holdings_with_prices: Option<Vec<(HoldingResponse, f64)>>, // (holding, current_price)
        frequency: String,
    ) -> ApiResult<()> {
        info!(
            "Sending immediate market data notification to user {} ({})",
            user_id, user_email
        );

        // Convert market data to notification format
        let market_symbols: Vec<MarketDataSymbol> = symbols_data
            .into_iter()
            .map(|s| {
                // Calculate change and change_percent from open price
                let change = s.price - s.open;
                let change_percent = if s.open == 0.0 {
                    0.0
                } else {
                    (change / s.open) * 100.0
                };

                MarketDataSymbol {
                    symbol: s.symbol,
                    price: s.price,
                    change,
                    change_percent,
                    high: s.high,
                    low: s.low,
                    volume: s.volume as i64,
                }
            })
            .collect();

        // Convert holdings to notification format with calculated metrics
        let holdings_data = holdings_with_prices.map(|holdings| {
            holdings
                .into_iter()
                .map(|(h, current_price)| {
                    let total_value = current_price * h.quantity as f64;
                    let cost_basis = h.average_cost * h.quantity as f64;
                    let profit_loss = total_value - cost_basis;
                    let profit_loss_percent = if cost_basis != 0.0 {
                        (profit_loss / cost_basis) * 100.0
                    } else {
                        0.0
                    };

                    HoldingData {
                        symbol: h.symbol,
                        quantity: h.quantity,
                        average_cost: h.average_cost,
                        current_price,
                        total_value,
                        profit_loss,
                        profit_loss_percent,
                    }
                })
                .collect()
        });

        // Create notification
        let notification = Notification::market_data_report(
            user_id,
            user_email,
            market_symbols,
            holdings_data,
            frequency,
        );

        // Publish to Redis channel for notification service
        let mut conn = self
            .get_connection()
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Redis connection error: {e}")))?;

        let notification_json = serde_json::to_string(&notification)
            .map_err(|e| ApiError::InternalServerError(format!("JSON serialization error: {e}")))?;

        conn.publish::<_, _, ()>("notifications", notification_json)
            .await
            .map_err(|e| {
                error!("Failed to publish notification to Redis: {}", e);
                ApiError::InternalServerError(format!("Failed to publish notification: {e}"))
            })?;

        info!("Market data notification published successfully");
        Ok(())
    }

    async fn get_connection(&self) -> Result<ConnectionManager, redis::RedisError> {
        ConnectionManager::new(self.redis_client.clone()).await
    }
}
