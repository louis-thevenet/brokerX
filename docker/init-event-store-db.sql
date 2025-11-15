-- Event Store Service Database Initialization

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Create order_events table
CREATE TABLE IF NOT EXISTS order_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL,
    user_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL CHECK (order_side IN ('buy', 'sell')),
    order_type JSONB NOT NULL,
    price DOUBLE PRECISION,
    filled_quantity BIGINT NOT NULL DEFAULT 0,
    average_price DOUBLE PRECISION,
    sequence_number BIGSERIAL,
    correlation_id UUID NOT NULL,
    causation_id UUID,
    metadata JSONB,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create saga_state table
CREATE TABLE IF NOT EXISTS saga_state (
    order_id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    current_step VARCHAR(50) NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL CHECK (order_side IN ('buy', 'sell')),
    order_type JSONB NOT NULL,
    price DOUBLE PRECISION,
    filled_quantity BIGINT,
    average_price DOUBLE PRECISION,
    correlation_id UUID NOT NULL,
    compensation_needed BOOLEAN NOT NULL DEFAULT FALSE,
    error_message TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

-- Create outbox table
CREATE TABLE IF NOT EXISTS outbox (
    outbox_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    processed_at TIMESTAMPTZ,
    processing_attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for order_events
CREATE INDEX IF NOT EXISTS idx_order_events_order_id ON order_events(order_id);
CREATE INDEX IF NOT EXISTS idx_order_events_user_id ON order_events(user_id);
CREATE INDEX IF NOT EXISTS idx_order_events_correlation_id ON order_events(correlation_id);
CREATE INDEX IF NOT EXISTS idx_order_events_occurred_at ON order_events(occurred_at DESC);

-- Create indexes for saga_state
CREATE INDEX IF NOT EXISTS idx_saga_state_user_id ON saga_state(user_id);
CREATE INDEX IF NOT EXISTS idx_saga_state_current_step ON saga_state(current_step);
CREATE INDEX IF NOT EXISTS idx_saga_state_correlation_id ON saga_state(correlation_id);

-- Create indexes for outbox
CREATE INDEX IF NOT EXISTS idx_outbox_order_id ON outbox(order_id);
CREATE INDEX IF NOT EXISTS idx_outbox_processed ON outbox(processed_at) WHERE processed_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_outbox_created_at ON outbox(created_at);

-- Create updated_at trigger for saga_state
CREATE OR REPLACE FUNCTION update_saga_state_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_saga_state_updated_at
    BEFORE UPDATE ON saga_state
    FOR EACH ROW
    EXECUTE FUNCTION update_saga_state_updated_at();

-- Grant permissions (database already created by Docker)
GRANT ALL PRIVILEGES ON TABLE order_events TO brokerx_user;
GRANT ALL PRIVILEGES ON TABLE saga_state TO brokerx_user;
GRANT ALL PRIVILEGES ON TABLE outbox TO brokerx_user;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO brokerx_user;

-- Log initialization
SELECT 'Event Store database initialized successfully' as status;
