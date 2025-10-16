-- Initial migration for market data service
-- Creates the market_prices table and necessary indexes

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
CREATE INDEX IF NOT EXISTS idx_market_prices_symbol ON market_prices(symbol);
CREATE INDEX IF NOT EXISTS idx_market_prices_timestamp ON market_prices(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_market_prices_symbol_timestamp ON market_prices(symbol, timestamp DESC);
