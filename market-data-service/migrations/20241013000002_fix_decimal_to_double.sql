-- Migration to fix column types from DECIMAL to DOUBLE PRECISION
-- This fixes the type mismatch between PostgreSQL NUMERIC and Rust f64

-- Alter the price columns from DECIMAL to DOUBLE PRECISION
ALTER TABLE market_prices 
    ALTER COLUMN price TYPE DOUBLE PRECISION,
    ALTER COLUMN open TYPE DOUBLE PRECISION,
    ALTER COLUMN high TYPE DOUBLE PRECISION,
    ALTER COLUMN low TYPE DOUBLE PRECISION;
