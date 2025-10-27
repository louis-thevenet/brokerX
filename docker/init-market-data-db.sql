-- Market Data Service Database Initialization
-- This script creates the schema for storing market price data

-- Create the market_prices table
CREATE TABLE IF NOT EXISTS market_prices (
    id UUID PRIMARY KEY,
    symbol VARCHAR(10) NOT NULL,
    price DOUBLE PRECISION NOT NULL CHECK (price > 0),
    open DOUBLE PRECISION NOT NULL CHECK (open > 0),
    high DOUBLE PRECISION NOT NULL CHECK (high > 0),
    low DOUBLE PRECISION NOT NULL CHECK (low > 0),
    volume BIGINT NOT NULL CHECK (volume >= 0),
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    CONSTRAINT price_range_check CHECK (low <= open AND low <= price AND low <= high),
    CONSTRAINT high_range_check CHECK (high >= open AND high >= price AND high >= low)
);

-- Create indexes for efficient querying
-- CRITICAL: Covering index for fast latest price lookups
CREATE INDEX IF NOT EXISTS idx_market_prices_symbol_timestamp_covering 
    ON market_prices(symbol, timestamp DESC) 
    INCLUDE (id, price, open, high, low, volume);

-- Fallback indexes
CREATE INDEX IF NOT EXISTS idx_market_prices_symbol ON market_prices(symbol);
CREATE INDEX IF NOT EXISTS idx_market_prices_timestamp ON market_prices(timestamp DESC);

-- Create the market_simulation_state table for tracking execution simulation state
CREATE TABLE IF NOT EXISTS market_simulation_state (
    symbol VARCHAR(10) PRIMARY KEY,
    current_volatility DOUBLE PRECISION NOT NULL DEFAULT 0.02 CHECK (current_volatility >= 0),
    trend_direction DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    last_execution_price DOUBLE PRECISION,
    last_execution_time TIMESTAMP WITH TIME ZONE,
    daily_volume BIGINT NOT NULL DEFAULT 1000000 CHECK (daily_volume >= 0),
    execution_count INTEGER NOT NULL DEFAULT 0 CHECK (execution_count >= 0),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index for efficient simulation state lookups
CREATE INDEX IF NOT EXISTS idx_market_simulation_state_symbol ON market_simulation_state(symbol);

-- Insert some initial sample data for testing
INSERT INTO market_prices (id, symbol, price, open, high, low, volume, timestamp)
VALUES 
    (gen_random_uuid(), 'AAPL', 175.50, 174.20, 176.80, 173.90, 1500000, NOW()),
    (gen_random_uuid(), 'GOOGL', 140.25, 139.50, 141.75, 138.90, 1200000, NOW()),
    (gen_random_uuid(), 'MSFT', 380.75, 379.30, 382.40, 378.50, 2000000, NOW()),
    (gen_random_uuid(), 'AMZN', 145.30, 144.80, 146.20, 144.10, 1800000, NOW()),
    (gen_random_uuid(), 'TSLA', 245.60, 243.90, 248.30, 242.50, 3000000, NOW()),
    (gen_random_uuid(), 'META', 325.40, 324.10, 327.80, 323.20, 1100000, NOW()),
    (gen_random_uuid(), 'NVDA', 485.90, 483.20, 489.50, 481.70, 2500000, NOW()),
    (gen_random_uuid(), 'NFLX', 455.20, 453.80, 458.90, 452.30, 900000, NOW())
ON CONFLICT DO NOTHING;

-- Grant necessary permissions
-- Uncomment if you need specific user permissions
-- GRANT SELECT, INSERT ON market_prices TO brokerx_user;
