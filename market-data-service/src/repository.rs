/// Market Data Repository
/// Handles all database operations for market price data
/// Follows the Repository pattern to abstract database access

use crate::models::MarketPrice;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Market price not found for symbol: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Clone)]
pub struct MarketDataRepository {
    pool: PgPool,
}

impl MarketDataRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the latest price for a specific symbol
    pub async fn get_latest_price(&self, symbol: &str) -> Result<MarketPrice> {
        let price = sqlx::query_as::<_, MarketPrice>(
            r#"
            SELECT id, symbol, price, open, high, low, volume, timestamp
            FROM market_prices
            WHERE symbol = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RepositoryError::NotFound(symbol.to_string()))?;

        Ok(price)
    }

    /// Get the latest prices for multiple symbols
    pub async fn get_latest_prices(&self, symbols: &[String]) -> Result<Vec<MarketPrice>> {
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        // Use DISTINCT ON to get the latest price for each symbol
        let prices = sqlx::query_as::<_, MarketPrice>(
            r#"
            SELECT DISTINCT ON (symbol) id, symbol, price, open, high, low, volume, timestamp
            FROM market_prices
            WHERE symbol = ANY($1)
            ORDER BY symbol, timestamp DESC
            "#,
        )
        .bind(symbols)
        .fetch_all(&self.pool)
        .await?;

        Ok(prices)
    }

    /// Get all available latest prices
    pub async fn get_all_latest_prices(&self) -> Result<Vec<MarketPrice>> {
        let prices = sqlx::query_as::<_, MarketPrice>(
            r#"
            SELECT DISTINCT ON (symbol) id, symbol, price, open, high, low, volume, timestamp
            FROM market_prices
            ORDER BY symbol, timestamp DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(prices)
    }

    /// Get historical prices for a symbol
    pub async fn get_historical_prices(
        &self,
        symbol: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<MarketPrice>> {
        let query = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                sqlx::query_as::<_, MarketPrice>(
                    r#"
                    SELECT id, symbol, price, open, high, low, volume, timestamp
                    FROM market_prices
                    WHERE symbol = $1 AND timestamp >= $2 AND timestamp <= $3
                    ORDER BY timestamp DESC
                    LIMIT $4
                    "#,
                )
                .bind(symbol)
                .bind(start)
                .bind(end)
                .bind(limit)
            }
            (Some(start), None) => {
                sqlx::query_as::<_, MarketPrice>(
                    r#"
                    SELECT id, symbol, price, open, high, low, volume, timestamp
                    FROM market_prices
                    WHERE symbol = $1 AND timestamp >= $2
                    ORDER BY timestamp DESC
                    LIMIT $3
                    "#,
                )
                .bind(symbol)
                .bind(start)
                .bind(limit)
            }
            (None, Some(end)) => {
                sqlx::query_as::<_, MarketPrice>(
                    r#"
                    SELECT id, symbol, price, open, high, low, volume, timestamp
                    FROM market_prices
                    WHERE symbol = $1 AND timestamp <= $2
                    ORDER BY timestamp DESC
                    LIMIT $3
                    "#,
                )
                .bind(symbol)
                .bind(end)
                .bind(limit)
            }
            (None, None) => {
                sqlx::query_as::<_, MarketPrice>(
                    r#"
                    SELECT id, symbol, price, open, high, low, volume, timestamp
                    FROM market_prices
                    WHERE symbol = $1
                    ORDER BY timestamp DESC
                    LIMIT $2
                    "#,
                )
                .bind(symbol)
                .bind(limit)
            }
        };

        let prices = query.fetch_all(&self.pool).await?;

        Ok(prices)
    }

    /// Insert a new market price record
    pub async fn insert_price(&self, price: &MarketPrice) -> Result<Uuid> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO market_prices (id, symbol, price, open, high, low, volume, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(&price.symbol)
        .bind(price.price)
        .bind(price.open)
        .bind(price.high)
        .bind(price.low)
        .bind(price.volume as i64)
        .bind(price.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get list of all available symbols
    pub async fn get_available_symbols(&self) -> Result<Vec<String>> {
        let symbols = sqlx::query_scalar::<_, String>(
            r#"
            SELECT DISTINCT symbol
            FROM market_prices
            ORDER BY symbol
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(symbols)
    }
}
