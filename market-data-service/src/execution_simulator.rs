/// Order Execution Simulator
/// Simulates realistic order execution without storing orders
/// Only updates market prices based on execution activity

use crate::models::{
    ExecuteOrderRequest, ExecuteOrderResponse, ExecutionStatus, MarketSimulationState, OrderSide,
    OrderType,
};
use crate::repository::{MarketDataRepository, RepositoryError};
use chrono::Utc;
use rand::Rng;
use tracing::{debug, info};

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Repository error: {0}")]
    RepositoryError(#[from] RepositoryError),
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
}

pub type Result<T> = std::result::Result<T, ExecutionError>;

pub struct ExecutionSimulator {
    repository: MarketDataRepository,
}

impl ExecutionSimulator {
    pub fn new(repository: MarketDataRepository) -> Self {
        Self { repository }
    }

    /// Simulate order execution based on current market conditions
    pub async fn execute_order(&self, request: ExecuteOrderRequest) -> Result<ExecuteOrderResponse> {
        let symbol = request.symbol.to_uppercase();
        debug!(
            "Processing order {} for {} ({:?}, {} shares)",
            request.order_id, symbol, request.order_type, request.quantity
        );

        // Get current market price
        let current_price = match self.repository.get_latest_price(&symbol).await {
            Ok(price) => {
                debug!("Current price for {}: ${:.2}", symbol, price.price);
                price.price
            }
            Err(e) => {
                debug!("Symbol {} not found: {}", symbol, e);
                return Ok(ExecuteOrderResponse {
                    order_id: request.order_id,
                    status: ExecutionStatus::Rejected,
                    filled_quantity: 0,
                    average_price: None,
                    message: Some(format!("Symbol {} not found", symbol)),
                    timestamp: Utc::now(),
                });
            }
        };

        // Get or create market simulation state
        let market_state = self
            .repository
            .get_or_create_market_state(&symbol, current_price)
            .await?;

        debug!(
            "Market state for {}: volatility={:.2}%, trend={:.2}",
            symbol,
            market_state.current_volatility * 100.0,
            market_state.trend_direction
        );

        // Validate order
        if request.quantity == 0 {
            debug!("Order {} rejected: invalid quantity", request.order_id);
            return Ok(ExecuteOrderResponse {
                order_id: request.order_id,
                status: ExecutionStatus::Rejected,
                filled_quantity: 0,
                average_price: None,
                message: Some("Invalid quantity: must be greater than 0".to_string()),
                timestamp: Utc::now(),
            });
        }

        // Simulate execution based on order type
        debug!("Simulating {:?} order execution", request.order_type);
        
        let execution = match request.order_type {
            OrderType::Market => {
                self.simulate_market_order(&request, current_price, &market_state)
                    .await?
            }
            OrderType::Limit { price } => {
                self.simulate_limit_order(&request, price, current_price, &market_state)
                    .await?
            }
        };

        // Update market state if order was filled
        if execution.status != ExecutionStatus::Rejected {
            self.update_market_after_execution(&symbol, &execution, &request)
                .await?;
        }

        // Log final result
        match execution.status {
            ExecutionStatus::FullyFilled => {
                debug!(
                    "Order {} fully filled: {} shares @ ${:.2}",
                    execution.order_id,
                    execution.filled_quantity,
                    execution.average_price.unwrap_or(0.0)
                );
            }
            ExecutionStatus::PartiallyFilled => {
                debug!(
                    "Order {} partially filled: {}/{} shares @ ${:.2}",
                    execution.order_id,
                    execution.filled_quantity,
                    request.quantity,
                    execution.average_price.unwrap_or(0.0)
                );
            }
            ExecutionStatus::Rejected => {
                debug!(
                    "Order {} rejected: {}",
                    execution.order_id,
                    execution.message.as_deref().unwrap_or("Unknown")
                );
            }
        }

        Ok(execution)
    }

    /// Simulate market order execution - typically fills immediately
    async fn simulate_market_order(
        &self,
        request: &ExecuteOrderRequest,
        current_price: f64,
        market_state: &MarketSimulationState,
    ) -> Result<ExecuteOrderResponse> {
        let mut rng = rand::thread_rng();

        debug!(
            "[MARKET ORDER] Order {} | Calculating execution for {} shares @ ${:.2}",
            request.order_id, request.quantity, current_price
        );

        // Calculate liquidity factor
        let liquidity_factor = self.calculate_liquidity_factor(request.quantity, market_state);
        debug!(
            "[LIQUIDITY] Order {} | Liquidity factor: {:.2} (0.0=low, 1.0=high)",
            request.order_id, liquidity_factor
        );

        // Market orders have high probability of execution
        let execution_probability = 0.95; // 95% chance of at least partial fill
        let roll = rng.gen_range(0.0..1.0);

        if roll > execution_probability {
            // Rejected (5% chance due to extreme conditions)
            debug!(
                "Order {} market order rejected due to insufficient liquidity",
                request.order_id
            );
            return Ok(ExecuteOrderResponse {
                order_id: request.order_id,
                status: ExecutionStatus::Rejected,
                filled_quantity: 0,
                average_price: None,
                message: Some("Insufficient market liquidity".to_string()),
                timestamp: Utc::now(),
            });
        }

        // Calculate slippage based on order size and volatility
        let slippage =
            self.calculate_slippage(request.quantity, market_state.current_volatility, liquidity_factor);
        
        debug!(
            "[SLIPPAGE] Order {} | Calculated slippage: {:.4}% (volatility: {:.2}%, liquidity: {:.2})",
            request.order_id, slippage * 100.0, market_state.current_volatility * 100.0, liquidity_factor
        );

        // Apply slippage (worse price for buyer, better for seller in thin markets)
        let execution_price = match request.side {
            OrderSide::Buy => current_price * (1.0 + slippage),
            OrderSide::Sell => current_price * (1.0 - slippage),
        };

        debug!(
            "[PRICE] Order {} | Base: ${:.2} -> Execution: ${:.2} ({:+.2}%)",
            request.order_id,
            current_price,
            execution_price,
            (execution_price - current_price) / current_price * 100.0
        );

        // Determine fill quantity based on liquidity
        let filled_quantity = if liquidity_factor < 0.5 {
            // Low liquidity = potential partial fill
            let fill_ratio = rng.gen_range(0.3..0.8);
            let qty = ((request.quantity as f64 * fill_ratio) as u64).max(1);
            debug!(
                "[PARTIAL FILL] Order {} | Low liquidity ({:.2}) -> Partial fill: {} of {} shares ({:.0}%)",
                request.order_id, liquidity_factor, qty, request.quantity, fill_ratio * 100.0
            );
            qty
        } else {
            // Good liquidity = full fill
            debug!(
                "[FULL FILL] Order {} | Good liquidity ({:.2}) -> Full fill: {} shares",
                request.order_id, liquidity_factor, request.quantity
            );
            request.quantity
        };

        let status = if filled_quantity == request.quantity {
            ExecutionStatus::FullyFilled
        } else {
            ExecutionStatus::PartiallyFilled
        };

        let rounded_price = (execution_price * 100.0).round() / 100.0;

        info!(
            "[MARKET EXEC] Order {} | {:?} | Filled {} shares @ ${:.2}",
            request.order_id, status, filled_quantity, rounded_price
        );

        Ok(ExecuteOrderResponse {
            order_id: request.order_id,
            status,
            filled_quantity,
            average_price: Some(rounded_price),
            message: None,
            timestamp: Utc::now(),
        })
    }

    /// Simulate limit order execution - only fills at limit price or better
    async fn simulate_limit_order(
        &self,
        request: &ExecuteOrderRequest,
        limit_price: f64,
        current_price: f64,
        _market_state: &MarketSimulationState,
    ) -> Result<ExecuteOrderResponse> {
        let mut rng = rand::thread_rng();

        debug!(
            "[LIMIT ORDER] Order {} | Limit: ${:.2} | Current: ${:.2} | Side: {:?}",
            request.order_id, limit_price, current_price, request.side
        );

        // Check if limit price is executable
        let is_executable = match request.side {
            OrderSide::Buy => limit_price >= current_price,
            OrderSide::Sell => limit_price <= current_price,
        };

        if !is_executable {
            // Price not reached - in real system would go into order book
            debug!(
                "Order {} limit price ${:.2} not reached (current ${:.2})",
                request.order_id, limit_price, current_price
            );
            return Ok(ExecuteOrderResponse {
                order_id: request.order_id,
                status: ExecutionStatus::Rejected,
                filled_quantity: 0,
                average_price: None,
                message: Some(format!(
                    "Limit price ${:.2} not reached (current ${:.2})",
                    limit_price, current_price
                )),
                timestamp: Utc::now(),
            });
        }

        debug!(
            "[EXECUTABLE] Order {} | Limit price ${:.2} is executable at current ${:.2}",
            request.order_id, limit_price, current_price
        );

        // Limit order at executable price
        // Simulate partial fill chance (30% for limit orders)
        let partial_fill_chance = 0.30;
        let roll = rng.gen_range(0.0..1.0);

        let filled_quantity = if roll < partial_fill_chance {
            let fill_ratio = rng.gen_range(0.4..0.9);
            let qty = ((request.quantity as f64 * fill_ratio) as u64).max(1);
            debug!(
                "⚠️  [PARTIAL] Order {} | Partial fill: {} of {} shares ({:.0}%)",
                request.order_id, qty, request.quantity, fill_ratio * 100.0
            );
            qty
        } else {
            debug!(
                "✓ [FULL] Order {} | Full fill: {} shares",
                request.order_id, request.quantity
            );
            request.quantity
        };

        let status = if filled_quantity == request.quantity {
            ExecutionStatus::FullyFilled
        } else {
            ExecutionStatus::PartiallyFilled
        };

        info!(
            "💰 [LIMIT EXEC] Order {} | {:?} | Filled {} shares @ ${:.2}",
            request.order_id, status, filled_quantity, limit_price
        );

        Ok(ExecuteOrderResponse {
            order_id: request.order_id,
            status,
            filled_quantity,
            average_price: Some(limit_price),
            message: None,
            timestamp: Utc::now(),
        })
    }

    /// Calculate liquidity factor (0.0 = no liquidity, 1.0 = perfect liquidity)
    fn calculate_liquidity_factor(&self, quantity: u64, market_state: &MarketSimulationState) -> f64 {
        // Estimate typical volume per second (assuming 6.5 hour trading day)
        let typical_volume_per_second = market_state.daily_volume as f64 / (6.5 * 3600.0);
        let order_impact = quantity as f64 / typical_volume_per_second.max(100.0);

        // Higher quantity relative to volume = lower liquidity
        // Keep between 0.2 and 1.0
        (1.0 - order_impact.min(0.8)).max(0.2)
    }

    /// Calculate slippage based on order size and market conditions
    fn calculate_slippage(&self, quantity: u64, volatility: f64, liquidity_factor: f64) -> f64 {
        let base_slippage = volatility * 0.1; // 10% of volatility
        let size_impact = (quantity as f64 / 1000.0).min(1.0) * 0.005; // Up to 0.5%
        let liquidity_impact = (1.0 - liquidity_factor) * 0.01; // Up to 1%

        (base_slippage + size_impact + liquidity_impact).min(0.05) // Cap at 5%
    }

    /// Update market state after order execution
    async fn update_market_after_execution(
        &self,
        symbol: &str,
        execution: &ExecuteOrderResponse,
        _request: &ExecuteOrderRequest,
    ) -> Result<()> {
        if let Some(exec_price) = execution.average_price {
            info!(
                "📝 [UPDATE STATE] {} | Updating market state: price=${:.2}, filled_quantity={}",
                symbol, exec_price, execution.filled_quantity
            );

            // Update market simulation state (adds to daily volume)
            self.repository
                .update_market_state(symbol, exec_price, execution.filled_quantity as i64)
                .await?;

            // Note: We don't insert execution prices as new market price points
            // because that would replace the market volume with the order quantity.
            // The regular price simulation updates will generate proper market prices.
            
            info!(
                "✅ [STATE UPDATED] {} | Market simulation state updated successfully",
                symbol
            );
        }

        Ok(())
    }
}
