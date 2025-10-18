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
