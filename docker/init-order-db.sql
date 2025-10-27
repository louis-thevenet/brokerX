-- Order Service Database Initialization  
-- This script runs when the order database container starts

-- Create orders table
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL,
    date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    symbol VARCHAR(20) NOT NULL,
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    order_side VARCHAR(10) NOT NULL CHECK (order_side IN ('Buy', 'Sell')),
    order_type_variant VARCHAR(20) NOT NULL CHECK (order_type_variant IN ('Market', 'Limit')),
    order_type_price DOUBLE PRECISION,
    status_variant VARCHAR(30) NOT NULL CHECK (status_variant IN ('Pending', 'PartiallyFilled', 'Filled', 'Rejected', 'Cancelled')),
    status_filled_quantity BIGINT,
    status_date TIMESTAMPTZ,
    status_reason TEXT,
    filled_price DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_orders_client_id ON orders(client_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status_variant);
CREATE INDEX IF NOT EXISTS idx_orders_client_status ON orders(client_id, status_variant);
CREATE INDEX IF NOT EXISTS idx_orders_client_date ON orders(client_id, date DESC);
CREATE INDEX IF NOT EXISTS idx_orders_symbol ON orders(symbol);

-- Create updated_at trigger
CREATE OR REPLACE FUNCTION update_orders_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_orders_updated_at
    BEFORE UPDATE ON orders
    FOR EACH ROW
    EXECUTE FUNCTION update_orders_updated_at();

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE brokerx_orders TO brokerx_user;
GRANT ALL PRIVILEGES ON TABLE orders TO brokerx_user;

-- Log initialization
SELECT 'Order database initialized successfully with orders table' as status;

