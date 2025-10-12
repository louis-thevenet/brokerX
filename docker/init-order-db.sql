-- Order Service Database Initialization  
-- This script runs when the order database container starts

-- Create additional schemas if needed
-- CREATE SCHEMA IF NOT EXISTS order_service;

-- Set up any initial data or configurations specific to order service
-- Example: Create indexes, constraints, etc.

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE brokerx_orders TO brokerx_user;

-- Log initialization
SELECT 'Order database initialized successfully' as status;
