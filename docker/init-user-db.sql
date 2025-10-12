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
