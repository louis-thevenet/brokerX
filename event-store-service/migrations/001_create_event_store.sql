-- Event Store Schema
-- This migration creates the core event store table for domain events

-- Events table - stores all domain events
CREATE TABLE IF NOT EXISTS events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    version BIGINT NOT NULL,
    occurred_at TIMESTAMP WITH TIME ZONE NOT NULL,
    stored_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    correlation_id UUID,
    causation_id UUID,
    metadata JSONB,
    
    -- Ensure version ordering per aggregate
    CONSTRAINT unique_aggregate_version UNIQUE (aggregate_id, version)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_events_aggregate_id ON events (aggregate_id);
CREATE INDEX IF NOT EXISTS idx_events_aggregate_type ON events (aggregate_type);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events (event_type);
CREATE INDEX IF NOT EXISTS idx_events_occurred_at ON events (occurred_at);
CREATE INDEX IF NOT EXISTS idx_events_stored_at ON events (stored_at);
CREATE INDEX IF NOT EXISTS idx_events_correlation_id ON events (correlation_id) WHERE correlation_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_events_causation_id ON events (causation_id) WHERE causation_id IS NOT NULL;

-- Composite index for aggregate queries
CREATE INDEX IF NOT EXISTS idx_events_aggregate_version ON events (aggregate_id, version);

-- Snapshots table for performance optimization (optional)
CREATE TABLE IF NOT EXISTS snapshots (
    aggregate_id UUID PRIMARY KEY,
    aggregate_type VARCHAR(100) NOT NULL,
    snapshot_data JSONB NOT NULL,
    version BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Index for snapshots
CREATE INDEX IF NOT EXISTS idx_snapshots_aggregate_type ON snapshots (aggregate_type);
CREATE INDEX IF NOT EXISTS idx_snapshots_created_at ON snapshots (created_at);
