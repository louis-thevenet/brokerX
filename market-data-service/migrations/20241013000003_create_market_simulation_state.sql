-- Market Simulation State Table
-- Stores state information for realistic order execution simulation

CREATE TABLE IF NOT EXISTS market_simulation_state (
    symbol VARCHAR(10) PRIMARY KEY,
    current_volatility DOUBLE PRECISION NOT NULL DEFAULT 0.02,
    trend_direction DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    last_execution_price DOUBLE PRECISION,
    last_execution_time TIMESTAMP WITH TIME ZONE,
    daily_volume BIGINT NOT NULL DEFAULT 0,
    execution_count INTEGER NOT NULL DEFAULT 0,
    
    CONSTRAINT volatility_range CHECK (current_volatility >= 0 AND current_volatility <= 1),
    CONSTRAINT trend_range CHECK (trend_direction >= -1 AND trend_direction <= 1)
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_market_simulation_symbol ON market_simulation_state(symbol);

-- Add comment for documentation
COMMENT ON TABLE market_simulation_state IS 'Tracks market simulation state for realistic order execution';
COMMENT ON COLUMN market_simulation_state.current_volatility IS 'Current market volatility (0.0 to 1.0)';
COMMENT ON COLUMN market_simulation_state.trend_direction IS 'Market trend direction (-1.0 bearish to +1.0 bullish)';
COMMENT ON COLUMN market_simulation_state.daily_volume IS 'Cumulative daily trading volume';
COMMENT ON COLUMN market_simulation_state.execution_count IS 'Number of orders executed today';
