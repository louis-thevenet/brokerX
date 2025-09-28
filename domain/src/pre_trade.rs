use std::collections::HashMap;

use crate::order::{OrderSide, OrderType};

/// Pre-trade validation errors
#[derive(Debug)]
pub enum PreTradeError {
    InsufficientBuyingPower {
        required: f64,
        available: f64,
    },
    InvalidPrice {
        reason: String,
    },
    ShortSellNotAllowed,
    ExceedsPositionLimit {
        limit: u64,
        requested: u64,
    },
    ExceedsNotionalLimit {
        limit: f64,
        requested: f64,
    },
    InvalidQuantity,
    InactiveInstrument {
        symbol: String,
    },
    InvalidTickSize {
        symbol: String,
        price: f64,
        tick_size: f64,
    },
    DbError(database_adapter::db::DbError),
}

impl std::fmt::Display for PreTradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreTradeError::InsufficientBuyingPower {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient buying power: required ${required:.2}, available ${available:.2}"
                )
            }
            PreTradeError::InvalidPrice { reason } => write!(f, "Invalid price: {reason}"),
            PreTradeError::ShortSellNotAllowed => write!(f, "Short selling not allowed"),
            PreTradeError::ExceedsPositionLimit { limit, requested } => {
                write!(
                    f,
                    "Exceeds position limit: limit {limit}, requested {requested}"
                )
            }
            PreTradeError::ExceedsNotionalLimit { limit, requested } => {
                write!(
                    f,
                    "Exceeds notional limit: limit ${limit:.2}, requested ${requested:.2}"
                )
            }
            PreTradeError::InvalidQuantity => write!(f, "Invalid quantity: must be greater than 0"),
            PreTradeError::InactiveInstrument { symbol } => {
                write!(f, "Instrument {symbol} is not active")
            }
            PreTradeError::InvalidTickSize {
                symbol,
                price,
                tick_size,
            } => {
                write!(
                    f,
                    "Invalid tick size for {symbol}: price {price:.4} not aligned to tick size {tick_size:.4}"
                )
            }
            PreTradeError::DbError(db_error) => {
                write!(f, "Database error: {db_error}")
            }
        }
    }
}

impl std::error::Error for PreTradeError {}

/// Configuration for pre-trade validation rules
#[derive(Debug, Clone)]
pub struct PreTradeConfig {
    pub max_position_size: u64,
    pub max_notional_per_order: f64,
    pub active_instruments: Vec<String>,
    pub tick_sizes: HashMap<String, f64>,
    pub price_bands: HashMap<String, (f64, f64)>, // (min, max)
}

impl Default for PreTradeConfig {
    fn default() -> Self {
        let mut tick_sizes = HashMap::new();
        tick_sizes.insert("AAPL".to_string(), 0.01);
        tick_sizes.insert("GOOGL".to_string(), 0.01);
        tick_sizes.insert("MSFT".to_string(), 0.01);
        tick_sizes.insert("TSLA".to_string(), 0.01);

        let mut price_bands = HashMap::new();
        price_bands.insert("AAPL".to_string(), (1.0, 1000.0));
        price_bands.insert("GOOGL".to_string(), (1.0, 5000.0));
        price_bands.insert("MSFT".to_string(), (1.0, 1000.0));
        price_bands.insert("TSLA".to_string(), (1.0, 2000.0));

        Self {
            max_position_size: 10000,
            max_notional_per_order: 100_000.0,
            active_instruments: vec![
                "AAPL".to_string(),
                "GOOGL".to_string(),
                "MSFT".to_string(),
                "TSLA".to_string(),
            ],
            tick_sizes,
            price_bands,
        }
    }
}

/// Pre-trade validation service
#[derive(Debug)]
pub struct PreTradeValidator {
    config: PreTradeConfig,
}

impl PreTradeValidator {
    pub fn new(config: PreTradeConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(PreTradeConfig::default())
    }

    /// Validates an order against pre-trade rules
    /// # Errors
    /// Returns `PreTradeError` if any validation fails
    pub fn validate_order(
        &self,
        order_side: &OrderSide,
        order_type: &OrderType,
        symbol: &str,
        quantity: u64,
        user_balance: f64,
    ) -> Result<(), PreTradeError> {
        // Sanity check: quantity > 0
        if quantity == 0 {
            return Err(PreTradeError::InvalidQuantity);
        }

        // Check if instrument is active
        if !self.config.active_instruments.contains(&symbol.to_string()) {
            return Err(PreTradeError::InactiveInstrument {
                symbol: symbol.to_string(),
            });
        }

        // Check position limits
        if quantity > self.config.max_position_size {
            return Err(PreTradeError::ExceedsPositionLimit {
                limit: self.config.max_position_size,
                requested: quantity,
            });
        }

        // Price validation for limit orders
        if let OrderType::Limit(price) = order_type {
            self.validate_limit_order_price(symbol, *price, quantity, order_side, user_balance)?;
        }

        // For market orders, validate with estimated prices
        if matches!(order_type, OrderType::Market) {
            self.validate_market_order(symbol, quantity, order_side, user_balance)?;
        }

        Ok(())
    }

    fn validate_limit_order_price(
        &self,
        symbol: &str,
        price: f64,
        quantity: u64,
        order_side: &OrderSide,
        user_balance: f64,
    ) -> Result<(), PreTradeError> {
        // Check price bands
        if let Some((min_price, max_price)) = self.config.price_bands.get(symbol) {
            if price < *min_price || price > *max_price {
                return Err(PreTradeError::InvalidPrice {
                    reason: format!(
                        "Price {price:.2} outside allowed band [{min_price:.2}, {max_price:.2}]"
                    ),
                });
            }
        }

        // Check tick size alignment
        if let Some(tick_size) = self.config.tick_sizes.get(symbol) {
            let remainder = (price / tick_size) % 1.0;
            if remainder.abs() > f64::EPSILON {
                return Err(PreTradeError::InvalidTickSize {
                    symbol: symbol.to_string(),
                    price,
                    tick_size: *tick_size,
                });
            }
        }

        // Notional value check
        let notional = price * (quantity as f64);
        if notional > self.config.max_notional_per_order {
            return Err(PreTradeError::ExceedsNotionalLimit {
                limit: self.config.max_notional_per_order,
                requested: notional,
            });
        }

        // Buying power check for buy orders
        if matches!(order_side, OrderSide::Buy) && notional > user_balance {
            return Err(PreTradeError::InsufficientBuyingPower {
                required: notional,
                available: user_balance,
            });
        }

        Ok(())
    }

    fn validate_market_order(
        &self,
        symbol: &str,
        quantity: u64,
        order_side: &OrderSide,
        user_balance: f64,
    ) -> Result<(), PreTradeError> {
        // Estimate with reasonable market price for basic checks
        let estimated_price = self.get_estimated_price(symbol);
        let estimated_notional = estimated_price * (quantity as f64);

        if estimated_notional > self.config.max_notional_per_order {
            return Err(PreTradeError::ExceedsNotionalLimit {
                limit: self.config.max_notional_per_order,
                requested: estimated_notional,
            });
        }

        if matches!(order_side, OrderSide::Buy) && estimated_notional > user_balance {
            return Err(PreTradeError::InsufficientBuyingPower {
                required: estimated_notional,
                available: user_balance,
            });
        }

        Ok(())
    }

    fn get_estimated_price(&self, symbol: &str) -> f64 {
        match symbol {
            "AAPL" => 150.0,
            "GOOGL" => 2800.0,
            "MSFT" => 420.0,
            "TSLA" => 245.0,
            _ => 100.0, // Default estimate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_limit_buy_order() {
        let validator = PreTradeValidator::with_default_config();
        let result = validator.validate_order(
            &OrderSide::Buy,
            &OrderType::Limit(150.50),
            "AAPL",
            100,
            20000.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_insufficient_buying_power() {
        let validator = PreTradeValidator::with_default_config();
        let result = validator.validate_order(
            &OrderSide::Buy,
            &OrderType::Limit(150.50),
            "AAPL",
            100,
            1000.0, // Not enough for 100 * 150.50 = 15,050
        );
        assert!(matches!(
            result,
            Err(PreTradeError::InsufficientBuyingPower { .. })
        ));
    }

    #[test]
    fn test_invalid_quantity() {
        let validator = PreTradeValidator::with_default_config();
        let result = validator.validate_order(
            &OrderSide::Buy,
            &OrderType::Limit(150.50),
            "AAPL",
            0, // Invalid quantity
            20000.0,
        );
        assert!(matches!(result, Err(PreTradeError::InvalidQuantity)));
    }

    #[test]
    fn test_inactive_instrument() {
        let validator = PreTradeValidator::with_default_config();
        let result = validator.validate_order(
            &OrderSide::Buy,
            &OrderType::Limit(50.0),
            "INVALID", // Not in active instruments
            100,
            20000.0,
        );
        assert!(matches!(
            result,
            Err(PreTradeError::InactiveInstrument { .. })
        ));
    }

    #[test]
    fn test_price_outside_bands() {
        let validator = PreTradeValidator::with_default_config();
        let result = validator.validate_order(
            &OrderSide::Buy,
            &OrderType::Limit(2000.0), // Outside AAPL band (1.0, 1000.0)
            "AAPL",
            100,
            300_000.0,
        );
        assert!(matches!(result, Err(PreTradeError::InvalidPrice { .. })));
    }
}
