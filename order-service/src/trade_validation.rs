//! Trade Validation Module
//!
//! Provides comprehensive pre-trade and post-trade validation checks

use crate::models::{OrderSide, OrderType};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct UserData {
    pub id: Option<Uuid>,
    pub email: String,
    pub balance: f64,
    pub is_verified: bool,
    pub holdings: std::collections::HashMap<String, HoldingData>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct HoldingData {
    pub symbol: String,
    pub quantity: u64,
    pub average_cost: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MarketPriceResponse {
    pub symbol: String,
    pub price: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: i64,
    pub timestamp: String, // ISO 8601 string from the API
}

/// Result of trade validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    #[must_use]
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationError {
    InsufficientFunds {
        required: f64,
        available: f64,
    },
    InsufficientHoldings {
        symbol: String,
        required: u64,
        available: u64,
    },
    InsufficientBuyingPower {
        required: f64,
        available: f64,
    },
    UserNotVerified {
        user_id: Uuid,
    },
    InvalidSymbol {
        symbol: String,
    },
    OrderTooLarge {
        quantity: u64,
        max_quantity: u64,
    },
    NotionalValueTooLarge {
        notional_value: f64,
        max_notional: f64,
    },
    OrderTooSmall {
        quantity: u64,
        min_quantity: u64,
    },
    PriceOutOfRange {
        price: f64,
        market_price: f64,
        max_deviation_percent: f64,
    },
    InvalidTickSize {
        price: f64,
        tick_size: f64,
    },
    ConcentrationLimit {
        symbol: String,
        new_position_percent: f64,
        max_position_percent: f64,
    },
    ShortSellingNotAllowed {
        symbol: String,
    },
    DailyOrderLimitExceeded {
        current_count: u32,
        max_count: u32,
    },
    InstrumentNotActive {
        symbol: String,
    },
    InvalidOrderData {
        message: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InsufficientFunds {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient funds: required ${required:.2}, available ${available:.2}"
                )
            }
            ValidationError::InsufficientHoldings {
                symbol,
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient holdings: need {required} shares of {symbol}, have {available} shares"
                )
            }
            ValidationError::InsufficientBuyingPower {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient buying power: required ${required:.2}, available ${available:.2}"
                )
            }
            ValidationError::UserNotVerified { user_id } => {
                write!(f, "User {user_id} is not verified")
            }
            ValidationError::InvalidSymbol { symbol } => {
                write!(f, "Invalid or unknown symbol: {symbol}")
            }
            ValidationError::OrderTooLarge {
                quantity,
                max_quantity,
            } => {
                write!(
                    f,
                    "Order size {quantity} exceeds maximum allowed: {max_quantity}"
                )
            }
            ValidationError::NotionalValueTooLarge {
                notional_value,
                max_notional,
            } => {
                write!(
                    f,
                    "Order notional value ${notional_value:.2} exceeds maximum allowed: ${max_notional:.2}"
                )
            }
            ValidationError::OrderTooSmall {
                quantity,
                min_quantity,
            } => {
                write!(
                    f,
                    "Order size {quantity} is below minimum required: {min_quantity}"
                )
            }
            ValidationError::PriceOutOfRange {
                price,
                market_price,
                max_deviation_percent,
            } => {
                write!(
                    f,
                    "Limit price ${price:.2} deviates more than {max_deviation_percent:.1}% from market price ${market_price:.2}"
                )
            }
            ValidationError::InvalidTickSize { price, tick_size } => {
                write!(
                    f,
                    "Price ${price:.4} does not conform to tick size ${tick_size:.4}"
                )
            }
            ValidationError::ConcentrationLimit {
                symbol,
                new_position_percent,
                max_position_percent,
            } => {
                write!(
                    f,
                    "Position in {symbol} would be {new_position_percent:.1}% of portfolio, exceeding limit of {max_position_percent:.1}%"
                )
            }
            ValidationError::ShortSellingNotAllowed { symbol } => {
                write!(f, "Short selling is not allowed for symbol {symbol}")
            }
            ValidationError::DailyOrderLimitExceeded {
                current_count,
                max_count,
            } => {
                write!(
                    f,
                    "Daily order limit exceeded: {current_count} orders today, maximum allowed: {max_count}"
                )
            }
            ValidationError::InstrumentNotActive { symbol } => {
                write!(f, "Instrument {symbol} is not active for trading")
            }
            ValidationError::InvalidOrderData { message } => {
                write!(f, "Invalid order data: {message}")
            }
        }
    }
}

/// Configuration for trade validation rules
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidationConfig {
    /// Maximum order size in shares
    pub max_order_quantity: u64,
    /// Maximum notional value per order (in dollars)
    pub max_notional_value: f64,
    /// Maximum percentage deviation for limit orders from market price
    pub max_price_deviation_percent: f64,
    /// Maximum position size as percentage of total portfolio value
    pub max_position_percent: f64,
    /// Minimum balance required after trade
    pub min_balance_after_trade: f64,
    /// Minimum tick size for price validation
    pub min_tick_size: f64,
    /// Enable short selling
    pub allow_short_selling: bool,
    /// Minimum order quantity
    pub min_order_quantity: u64,
    /// Maximum daily order count per user
    pub max_daily_orders: u32,
    /// Buying power multiplier (1.0 = cash only, 2.0 = 2x margin)
    pub buying_power_multiplier: f64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_order_quantity: 10_000,
            max_notional_value: 100_000.0, // $100k per order
            max_price_deviation_percent: 10.0,
            max_position_percent: 25.0,
            min_balance_after_trade: 0.0,
            min_tick_size: 0.01,        // $0.01 tick size
            allow_short_selling: false, // Disabled by default for retail
            min_order_quantity: 1,
            max_daily_orders: 100,
            buying_power_multiplier: 1.0, // Cash account only
        }
    }
}

/// Trade validator that performs pre-trade and post-trade checks
pub struct TradeValidator {
    http_client: Arc<Client>,
    user_service_url: String,
    market_data_service_url: String,
    config: ValidationConfig,
}

impl TradeValidator {
    pub fn new(
        http_client: Arc<Client>,
        user_service_url: String,
        market_data_service_url: String,
        config: ValidationConfig,
    ) -> Self {
        Self {
            http_client,
            user_service_url,
            market_data_service_url,
            config,
        }
    }

    /// Get user data from user service
    async fn get_user(&self, user_id: Uuid) -> Result<UserData, String> {
        let url = format!("{}/v1/users/{}", self.user_service_url, user_id);
        self.http_client
            .get(&url)
            .timeout(std::time::Duration::from_millis(200))
            .send()
            .await
            .map_err(|e| format!("Failed to get user: {e}"))?
            .json::<UserData>()
            .await
            .map_err(|e| format!("Failed to parse user data: {e}"))
    }

    /// Get market price from market data service
    async fn get_price(&self, symbol: &str) -> Result<f64, String> {
        let url = format!("{}/v1/prices/{}", self.market_data_service_url, symbol);
        let price_data = self
            .http_client
            .get(&url)
            .timeout(std::time::Duration::from_millis(200))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch price: {e}"))?
            .json::<MarketPriceResponse>()
            .await
            .map_err(|e| format!("Failed to parse price data: {e}"))?;
        Ok(price_data.price)
    }

    /// Perform pre-trade validation
    pub async fn validate_trade(
        &self,
        user_id: Uuid,
        symbol: &str,
        quantity: u64,
        order_side: &OrderSide,
        order_type: &OrderType,
    ) -> ValidationResult {
        debug!(
            "Validating trade: user={}, symbol={}, qty={}, side={:?}",
            user_id, symbol, quantity, order_side
        );

        let mut result = ValidationResult::valid();

        // 1. Sanity check: quantity must be > 0
        if quantity == 0 {
            result.add_error(ValidationError::InvalidOrderData {
                message: "Quantity must be greater than 0".to_string(),
            });
            return result;
        }

        // 2. Validate order size (min and max)
        if let Err(e) = self.validate_order_size(quantity) {
            result.add_error(e);
        }

        // 3. Get user data
        let user = match self.get_user(user_id).await {
            Ok(user) => user,
            Err(e) => {
                warn!("Failed to fetch user {}: {}", user_id, e);
                result.add_error(ValidationError::InvalidOrderData {
                    message: "Unable to verify user data".to_string(),
                });
                return result;
            }
        };

        // 4. Validate user is verified
        if !user.is_verified {
            result.add_error(ValidationError::UserNotVerified { user_id });
        }

        // 5. Get market price and validate symbol is active
        let market_price = match self.get_price(symbol).await {
            Ok(price) => price,
            Err(e) => {
                warn!("Failed to get price for symbol {}: {}", symbol, e);
                result.add_error(ValidationError::InvalidSymbol {
                    symbol: symbol.to_string(),
                });
                return result;
            }
        };

        // 6. Validate limit price if applicable
        if let OrderType::Limit { price } = order_type {
            // Validate tick size
            if let Err(e) = self.validate_tick_size(*price) {
                result.add_error(e);
            }

            // Validate price is within acceptable bands
            if let Err(e) = self.validate_limit_price(*price, market_price, order_side) {
                result.add_error(e);
            }
        }

        // 7. Validate funds/holdings based on order side
        match order_side {
            OrderSide::Buy => {
                let estimated_cost =
                    self.calculate_estimated_cost(quantity, market_price, order_type);

                // Check notional value limits
                if let Err(e) = self.validate_notional_value(estimated_cost) {
                    result.add_error(e);
                }

                // Check buying power (includes margin)
                if let Err(e) = self.validate_buying_power(&user, estimated_cost) {
                    result.add_error(e);
                }

                // Check concentration limits for buy orders
                if let Err(e) =
                    self.validate_concentration_limit(&user, symbol, quantity, market_price)
                {
                    result.add_error(e);
                }
            }
            OrderSide::Sell => {
                // Check if user has sufficient holdings
                if let Err(e) = self.validate_holdings(&user, symbol, quantity) {
                    result.add_error(e);
                } else {
                    // If user doesn't have holdings, check if short selling is allowed
                    let current_holdings = user.holdings.get(symbol).map_or(0, |h| h.quantity);

                    if current_holdings == 0 && !self.config.allow_short_selling {
                        result.add_error(ValidationError::ShortSellingNotAllowed {
                            symbol: symbol.to_string(),
                        });
                    }
                }
            }
        }

        debug!(
            "Validation result for user {}: valid={}, errors={}",
            user_id,
            result.valid,
            result.errors.len()
        );

        result
    }

    fn validate_order_size(&self, quantity: u64) -> Result<(), ValidationError> {
        if quantity < self.config.min_order_quantity {
            return Err(ValidationError::OrderTooSmall {
                quantity,
                min_quantity: self.config.min_order_quantity,
            });
        }

        if quantity > self.config.max_order_quantity {
            return Err(ValidationError::OrderTooLarge {
                quantity,
                max_quantity: self.config.max_order_quantity,
            });
        }
        Ok(())
    }

    fn validate_notional_value(&self, notional_value: f64) -> Result<(), ValidationError> {
        if notional_value > self.config.max_notional_value {
            return Err(ValidationError::NotionalValueTooLarge {
                notional_value,
                max_notional: self.config.max_notional_value,
            });
        }
        Ok(())
    }

    fn validate_tick_size(&self, price: f64) -> Result<(), ValidationError> {
        let remainder = (price / self.config.min_tick_size).fract();
        // Allow small floating point errors
        if remainder.abs() > 0.0001 && (1.0 - remainder).abs() > 0.0001 {
            return Err(ValidationError::InvalidTickSize {
                price,
                tick_size: self.config.min_tick_size,
            });
        }
        Ok(())
    }

    fn validate_buying_power(
        &self,
        user: &UserData,
        required_amount: f64,
    ) -> Result<(), ValidationError> {
        let buying_power = user.balance * self.config.buying_power_multiplier;

        if buying_power < required_amount {
            return Err(ValidationError::InsufficientBuyingPower {
                required: required_amount,
                available: buying_power,
            });
        }

        // Check minimum balance after trade (for cash accounts)
        if self.config.buying_power_multiplier == 1.0 {
            let balance_after = user.balance - required_amount;
            if balance_after < self.config.min_balance_after_trade {
                return Err(ValidationError::InsufficientFunds {
                    required: required_amount + self.config.min_balance_after_trade,
                    available: user.balance,
                });
            }
        }

        Ok(())
    }

    fn validate_holdings(
        &self,
        user: &UserData,
        symbol: &str,
        required_quantity: u64,
    ) -> Result<(), ValidationError> {
        let available = user.holdings.get(symbol).map_or(0, |h| h.quantity);

        if available < required_quantity {
            return Err(ValidationError::InsufficientHoldings {
                symbol: symbol.to_string(),
                required: required_quantity,
                available,
            });
        }

        Ok(())
    }

    fn validate_limit_price(
        &self,
        limit_price: f64,
        market_price: f64,
        order_side: &OrderSide,
    ) -> Result<(), ValidationError> {
        let deviation_percent = ((limit_price - market_price).abs() / market_price) * 100.0;

        if deviation_percent > self.config.max_price_deviation_percent {
            let is_problematic = match order_side {
                OrderSide::Buy => {
                    limit_price
                        > market_price * (1.0 + self.config.max_price_deviation_percent / 100.0)
                }
                OrderSide::Sell => {
                    limit_price
                        < market_price * (1.0 - self.config.max_price_deviation_percent / 100.0)
                }
            };

            if is_problematic {
                return Err(ValidationError::PriceOutOfRange {
                    price: limit_price,
                    market_price,
                    max_deviation_percent: self.config.max_price_deviation_percent,
                });
            }
        }

        Ok(())
    }

    fn validate_concentration_limit(
        &self,
        user: &UserData,
        symbol: &str,
        additional_quantity: u64,
        market_price: f64,
    ) -> Result<(), ValidationError> {
        let mut portfolio_value = user.balance;
        for holding in user.holdings.values() {
            portfolio_value += holding.quantity as f64 * holding.average_cost;
        }

        if portfolio_value <= 0.0 {
            return Ok(());
        }

        let current_quantity = user.holdings.get(symbol).map_or(0, |h| h.quantity);

        let new_total_quantity = current_quantity + additional_quantity;
        let new_position_value = new_total_quantity as f64 * market_price;
        let new_position_percent = (new_position_value / portfolio_value) * 100.0;

        if new_position_percent > self.config.max_position_percent {
            return Err(ValidationError::ConcentrationLimit {
                symbol: symbol.to_string(),
                new_position_percent,
                max_position_percent: self.config.max_position_percent,
            });
        }

        Ok(())
    }

    fn calculate_estimated_cost(
        &self,
        quantity: u64,
        market_price: f64,
        order_type: &OrderType,
    ) -> f64 {
        let price = match order_type {
            OrderType::Market => market_price,
            OrderType::Limit { price } => *price,
        };

        quantity as f64 * price
    }
}
