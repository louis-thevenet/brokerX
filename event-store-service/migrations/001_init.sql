-- Event Store Service Database Schema
-- This schema implements Event-Driven Architecture for Order Processing (UC-07)
-- Using Saga Choreography Pattern

-- ============================================================================
-- ORDER EVENTS TABLE
-- Stores all events in the order processing saga
-- ============================================================================
CREATE TABLE IF NOT EXISTS order_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL,
    user_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    -- Event types: OrderCreated, OrderValidated, OrderRejected, OrderExecuted, 
    --              OrderFilled, OrderPartiallyFilled, FundsDebited, FundsCredited,
    --              HoldingsUpdated, OrderCompleted
    
    -- Order details (for saga context)
    symbol VARCHAR(10) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL,  -- 'buy' or 'sell'
    order_type VARCHAR(20) NOT NULL,  -- 'market' or 'limit'
    price DECIMAL(15, 2),              -- For limit orders
    
    -- Execution details (populated during execution)
    filled_quantity BIGINT DEFAULT 0,
    average_price DECIMAL(15, 2),
    
    -- Event metadata
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_number BIGSERIAL,
    
    -- Saga correlation
    correlation_id UUID,  -- For tracing the entire saga
    causation_id UUID,    -- Which event caused this event
    
    -- Additional data (errors, validation results, etc.)
    metadata JSONB,
    
    CONSTRAINT order_events_sequence_unique UNIQUE (sequence_number)
);

-- Index for querying all events of an order (rebuilding order state)
CREATE INDEX idx_order_events_order ON order_events(order_id, sequence_number);

-- Index for querying events by type (e.g., all failed validations)
CREATE INDEX idx_order_events_type ON order_events(event_type, occurred_at);

-- Index for user's order events
CREATE INDEX idx_order_events_user ON order_events(user_id, occurred_at);

-- Index for correlation tracking (tracing sagas)
CREATE INDEX idx_order_events_correlation ON order_events(correlation_id) WHERE correlation_id IS NOT NULL;

-- ============================================================================
-- OUTBOX TABLE
-- Transactional Outbox Pattern for reliable event publishing
-- ============================================================================
CREATE TABLE IF NOT EXISTS outbox (
    outbox_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    processing_attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    
    CONSTRAINT outbox_processing CHECK (
        (processed_at IS NULL AND processing_attempts >= 0) OR
        (processed_at IS NOT NULL)
    )
);

-- Index for polling unprocessed messages
CREATE INDEX idx_outbox_unprocessed ON outbox(created_at) 
WHERE processed_at IS NULL;

-- Index for monitoring failed messages
CREATE INDEX idx_outbox_failed ON outbox(processing_attempts, created_at) 
WHERE processed_at IS NULL AND processing_attempts > 3;

-- ============================================================================
-- SAGA STATE TABLE
-- Tracks the current state of each order saga
-- ============================================================================
CREATE TABLE IF NOT EXISTS saga_state (
    order_id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    current_step VARCHAR(50) NOT NULL,
    -- Steps: CREATED, VALIDATING, VALIDATED, EXECUTING, EXECUTED, 
    --        SETTLING, COMPLETED, FAILED, COMPENSATING, COMPENSATED
    
    symbol VARCHAR(10) NOT NULL,
    quantity BIGINT NOT NULL,
    order_side VARCHAR(10) NOT NULL,
    order_type VARCHAR(20) NOT NULL,
    price DECIMAL(15, 2),
    
    -- Saga execution tracking
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    
    -- Results
    filled_quantity BIGINT DEFAULT 0,
    average_price DECIMAL(15, 2),
    
    -- Error tracking for compensation
    error_message TEXT,
    compensation_needed BOOLEAN DEFAULT FALSE,
    
    -- Saga correlation
    correlation_id UUID NOT NULL,
    
    metadata JSONB
);

-- Index for active sagas
CREATE INDEX idx_saga_state_active ON saga_state(updated_at) 
WHERE completed_at IS NULL;

-- Index for failed sagas needing compensation
CREATE INDEX idx_saga_state_compensation ON saga_state(started_at) 
WHERE compensation_needed = TRUE AND completed_at IS NULL;

-- Index for user's sagas
CREATE INDEX idx_saga_state_user ON saga_state(user_id, started_at DESC);

-- ============================================================================
-- SUBSCRIPTIONS TABLE
-- Tracks event stream positions for consumers
-- ============================================================================
CREATE TABLE IF NOT EXISTS subscriptions (
    subscription_id VARCHAR(100) PRIMARY KEY,
    last_processed_sequence BIGINT NOT NULL DEFAULT 0,
    last_processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Auto-update updated_at for saga_state
CREATE OR REPLACE FUNCTION update_saga_state_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_saga_state_updated_at 
    BEFORE UPDATE ON saga_state
    FOR EACH ROW
    EXECUTE FUNCTION update_saga_state_timestamp();

-- Auto-update updated_at for subscriptions
CREATE OR REPLACE FUNCTION update_subscriptions_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_subscriptions_updated_at 
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_subscriptions_timestamp();

