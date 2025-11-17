-- User Service Database Initialization
-- This script runs when the user database container starts

-- Create additional schemas if needed
-- CREATE SCHEMA IF NOT EXISTS user_service;

-- Set up any initial data or configurations specific to user service
-- Example: Create indexes, constraints, etc.

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE brokerx_users TO brokerx_user;

-- Log initialization
SELECT 'User database initialized successfully' as status;

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    firstname TEXT NOT NULL,
    surname TEXT NOT NULL,
    balance DOUBLE PRECISION NOT NULL,
    is_verified BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    holdings JSONB NOT NULL
);

-- Ensure unique index on email
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Optional: index for is_verified if you filter by it often
-- CREATE INDEX IF NOT EXISTS idx_users_is_verified ON users(is_verified);

-- Create market data subscriptions table
CREATE TABLE IF NOT EXISTS market_data_subscriptions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbols TEXT[] NOT NULL, -- Array of symbols user is subscribed to
    frequency TEXT NOT NULL CHECK (frequency IN ('hourly', 'daily', 'weekly')),
    include_holdings BOOLEAN NOT NULL DEFAULT true, -- Include user's holdings in reports
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_sent_at TIMESTAMPTZ
);

-- Index for faster user subscription lookups
CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON market_data_subscriptions(user_id);

-- Index for active subscriptions by frequency (for scheduled tasks)
CREATE INDEX IF NOT EXISTS idx_subscriptions_frequency_active ON market_data_subscriptions(frequency, is_active) WHERE is_active = true;
