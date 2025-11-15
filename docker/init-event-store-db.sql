-- Create Event Store Database
CREATE DATABASE brokerx_events;

-- Connect to the database
\c brokerx_events;

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create user for event store service
CREATE USER event_store_user WITH PASSWORD 'event_store_pass';

-- Grant privileges
GRANT ALL PRIVILEGES ON DATABASE brokerx_events TO event_store_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO event_store_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO event_store_user;

-- Ensure future tables are also granted
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO event_store_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO event_store_user;
