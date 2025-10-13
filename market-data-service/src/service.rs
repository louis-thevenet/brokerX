/// Market Data Service
/// Business logic layer for market data operations
/// Implements price simulation and data management

use crate::models::{HistoricalPricesResponse, MarketPrice, MarketPriceResponse, MarketPricesResponse};
use crate::repository::{MarketDataRepository, RepositoryError};
use chrono::{DateTime, Utc};
use rand::Rng;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Repository error: {0}")]
    RepositoryError(#[from] RepositoryError),
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
    #[error("Invalid price data")]
    InvalidPriceData,
}

pub type Result<T> = std::result::Result<T, ServiceError>;

pub struct MarketDataService {
    repository: MarketDataRepository,
}

impl MarketDataService {
    pub fn new(repository: MarketDataRepository) -> Self {
        Self { repository }
    }

    /// Get the current (latest) price for a symbol
    pub async fn get_current_price(&self, symbol: &str) -> Result<MarketPriceResponse> {
        let symbol = symbol.to_uppercase();
        let price = self.repository.get_latest_price(&symbol).await?;
        Ok(price.into())
    }

    /// Get current prices for multiple symbols
    pub async fn get_current_prices(&self, symbols: Vec<String>) -> Result<MarketPricesResponse> {
        let symbols: Vec<String> = symbols.iter().map(|s| s.to_uppercase()).collect();
        let prices = self.repository.get_latest_prices(&symbols).await?;
        
        let price_responses: Vec<MarketPriceResponse> = prices.into_iter().map(|p| p.into()).collect();
        let count = price_responses.len();

        Ok(MarketPricesResponse {
            prices: price_responses,
            count,
        })
    }

    /// Get all available current prices
    pub async fn get_all_current_prices(&self) -> Result<MarketPricesResponse> {
        let prices = self.repository.get_all_latest_prices().await?;
        
        let price_responses: Vec<MarketPriceResponse> = prices.into_iter().map(|p| p.into()).collect();
        let count = price_responses.len();

        Ok(MarketPricesResponse {
            prices: price_responses,
            count,
        })
    }

    /// Get historical prices for a symbol
    pub async fn get_historical_prices(
        &self,
        symbol: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<HistoricalPricesResponse> {
        let symbol = symbol.to_uppercase();
        let prices = self
            .repository
            .get_historical_prices(&symbol, start_date, end_date, limit)
            .await?;

        let price_responses: Vec<MarketPriceResponse> = prices.into_iter().map(|p| p.into()).collect();
        let count = price_responses.len();

        Ok(HistoricalPricesResponse {
            symbol,
            prices: price_responses,
            count,
        })
    }

    /// Get list of all available symbols
    pub async fn get_available_symbols(&self) -> Result<Vec<String>> {
        let symbols = self.repository.get_available_symbols().await?;
        Ok(symbols)
    }

    /// Simulate and generate new market prices for all symbols
    /// This is a simple simulation for demonstration purposes
    pub async fn simulate_price_update(&self) -> Result<usize> {
        let symbols = self.repository.get_available_symbols().await?;
        let mut updated_count = 0;

        for symbol in symbols {
            if let Ok(latest) = self.repository.get_latest_price(&symbol).await {
                let new_price = self.simulate_price_change(&latest);
                self.repository.insert_price(&new_price).await?;
                updated_count += 1;
            }
        }

        Ok(updated_count)
    }

    /// Simulate a realistic price change based on the previous price
    fn simulate_price_change(&self, previous: &MarketPrice) -> MarketPrice {
        let mut rng = rand::thread_rng();
        
        // Simulate price change between -2% and +2%
        let change_percent = rng.gen_range(-0.02..0.02);
        let new_price = previous.price * (1.0 + change_percent);
        
        // Calculate new open, high, low
        let volatility = rng.gen_range(-0.01..0.01);
        let open = previous.price * (1.0 + volatility);
        let high = new_price.max(open) * (1.0 + rng.gen_range(0.0..0.005));
        let low = new_price.min(open) * (1.0 - rng.gen_range(0.0..0.005));
        
        // Simulate volume change
        let volume_change = rng.gen_range(0.8..1.2);
        let volume = (previous.volume as f64 * volume_change) as i64;

        MarketPrice {
            id: None,
            symbol: previous.symbol.clone(),
            price: (new_price * 100.0).round() / 100.0, // Round to 2 decimal places
            open: (open * 100.0).round() / 100.0,
            high: (high * 100.0).round() / 100.0,
            low: (low * 100.0).round() / 100.0,
            volume,
            timestamp: Utc::now(),
        }
    }

    /// Initialize market data with some predefined symbols
    /// This is useful for first-time setup or testing
    pub async fn initialize_market_data(&self) -> Result<usize> {
        let initial_symbols = vec![
            ("AAPL", 175.50, 1_500_000_i64),
            ("GOOGL", 140.25, 1_200_000_i64),
            ("MSFT", 380.75, 2_000_000_i64),
            ("AMZN", 145.30, 1_800_000_i64),
            ("TSLA", 245.60, 3_000_000_i64),
            ("META", 325.40, 1_100_000_i64),
            ("NVDA", 485.90, 2_500_000_i64),
            ("NFLX", 455.20, 900_000_i64),
        ];

        let mut created_count = 0;

        for (symbol, base_price, volume) in initial_symbols {
            // Check if symbol already exists
            if self.repository.get_latest_price(symbol).await.is_ok() {
                continue;
            }

            // Generate all random values before any await points to avoid Send issues
            let (price, open, high, low) = {
                let mut rng = rand::thread_rng();
                let price_variation = rng.gen_range(-0.02..0.02);
                let price: f64 = base_price * (1.0 + price_variation);
                let open: f64 = base_price * (1.0 + rng.gen_range(-0.01..0.01));
                let high: f64 = price.max(open) * (1.0 + rng.gen_range(0.0..0.01));
                let low: f64 = price.min(open) * (1.0 - rng.gen_range(0.0..0.01));
                (price, open, high, low)
            };

            let market_price = MarketPrice {
                id: None,
                symbol: symbol.to_string(),
                price: (price * 100.0).round() / 100.0,
                open: (open * 100.0).round() / 100.0,
                high: (high * 100.0).round() / 100.0,
                low: (low * 100.0).round() / 100.0,
                volume,
                timestamp: Utc::now(),
            };

            self.repository.insert_price(&market_price).await?;
            created_count += 1;
        }

        Ok(created_count)
    }
}
