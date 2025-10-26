# Performance Optimizations for 800 Orders/Second

This document describes the optimizations implemented to stabilize the system at 800 orders/second throughput.

## Problem Statement

The system was reaching 800 orders/s but becoming unstable with endpoints timing out:
- `/v1/orders` P99: 235.929 ms (degrading)
- `/v1/orders/{id}/execution` P99: 92.105 ms
- `/v1/users/{id}/withdraw` P99: 46.986 ms
- `/v1/users/{id}/deposit` P99: 45.722 ms

## Root Causes Identified

1. **Database Connection Pool Exhaustion**: 100 max connections insufficient for high concurrency
2. **HTTP Client Pool Limitations**: 50 idle connections per host too conservative
3. **Long HTTP Timeouts**: 30-second timeouts causing resource exhaustion
4. **PostgreSQL Default Settings**: Not optimized for write-heavy workloads
5. **Excessive Concurrency**: Orchestrator allowing 1000 concurrent tasks overwhelming downstream services

## Optimizations Implemented

### 1. Database Connection Pool Tuning

#### User Service (`user-service/src/main.rs`)
```rust
PgPoolOptions::new()
    .max_connections(200)           // Increased from 100
    .min_connections(20)            // Keep connections warm
    .acquire_timeout(5s)            // Fail fast on exhaustion
    .idle_timeout(300s)             // 5 min idle timeout
    .max_lifetime(1800s)            // 30 min max lifetime
```

#### Order Service (`order-service/src/service.rs`)
```rust
PgPoolOptions::new()
    .max_connections(250)           // Highest - most write-heavy
    .min_connections(25)
    .acquire_timeout(5s)
    .idle_timeout(300s)
    .max_lifetime(1800s)
```

#### Market Data Service (`market-data-service/src/main.rs`)
```rust
PgPoolOptions::new()
    .max_connections(150)           // Moderate - mostly reads
    .min_connections(15)
    .acquire_timeout(5s)
    .idle_timeout(300s)
    .max_lifetime(1800s)
```

### 2. HTTP Client Optimization

#### API Gateway (`api_gateway/src/clients.rs`)
```rust
fn create_optimized_client() -> Client {
    Client::builder()
        .timeout(10s)                           // Reduced from 30s
        .connect_timeout(3s)                    // Reduced from 10s
        .pool_max_idle_per_host(100)           // Increased from 50
        .pool_idle_timeout(90s)                // Keep connections alive
        .tcp_keepalive(60s)                    // TCP keepalive
        .http2_keep_alive_interval(10s)        // HTTP/2 keepalive
        .http2_keep_alive_timeout(20s)
        .build()
}
```

Applied to all clients: UserClient, OrderClient, MarketDataClient, MarketDataExecutionClient, OrchestratorClient

#### Orchestrator Service (`order-execution-orchestrator-service/src/clients.rs`)
Already optimized with:
- 500ms timeout for user service (fast reads)
- 2s timeout for API Gateway (allows cache misses)
- Connection pooling with 100 idle connections per host

### 3. Concurrency Limiting

#### Orchestrator Service (`order-execution-orchestrator-service/src/main.rs`)
```rust
// Reduced from 1000 to 300
let concurrency_limiter = Arc::new(Semaphore::new(300));
```

This prevents overwhelming downstream services while maintaining high throughput.

### 4. PostgreSQL Tuning

#### Order Database (docker-compose.yml)
Most write-heavy service - aggressive tuning:
```yaml
max_connections: 400
shared_buffers: 512MB
effective_cache_size: 2GB
work_mem: 2MB
min_wal_size: 2GB
max_wal_size: 8GB
synchronous_commit: off              # Trading durability for performance
checkpoint_completion_target: 0.9
effective_io_concurrency: 200
```

#### User Database
```yaml
max_connections: 300
shared_buffers: 256MB
effective_cache_size: 1GB
work_mem: 2MB
min_wal_size: 1GB
max_wal_size: 4GB
```

#### Market Data Database
```yaml
max_connections: 250
shared_buffers: 256MB
effective_cache_size: 1GB
work_mem: 2MB
min_wal_size: 1GB
max_wal_size: 4GB
```

### 5. Container Resource Limits

Added resource limits to prevent CPU/memory contention:

```yaml
# API Gateway & Orchestrator (high load)
resources:
  limits:
    cpus: '2.0'
    memory: 1G
  reservations:
    cpus: '0.5'
    memory: 512M

# Order & User Services (high load)
resources:
  limits:
    cpus: '2.0'
    memory: 1G
  reservations:
    cpus: '0.5'
    memory: 512M

# Market Data Service (moderate load)
resources:
  limits:
    cpus: '1.5'
    memory: 768M
  reservations:
    cpus: '0.25'
    memory: 256M
```

### 6. Cache Optimization

#### API Gateway Price Cache (`api_gateway/src/main.rs`)
```rust
Cache::builder()
    .max_capacity(200)              // Increased from 100
    .time_to_live(2500ms)           // Optimized for 3s update cycle
    .initial_capacity(100)          // Pre-allocate
    .build()
```

## Expected Improvements

### Latency Reduction
- **Order Creation**: P99 < 100ms (from 235ms)
- **Order Execution**: P99 < 50ms (from 92ms)
- **Deposits/Withdrawals**: P99 < 30ms (from 45ms)
- **Holdings Queries**: P99 < 20ms (stable)

### Throughput Stability
- **Sustained 800 orders/s** without degradation
- Better handling of traffic spikes
- Reduced timeout errors
- Lower database connection contention

### Resource Utilization
- More efficient connection pooling
- Better CPU distribution across services
- Reduced memory pressure
- Lower network latency

## Testing & Validation

Run the quality validation test:
```bash
cd load-tests
./run-tests.fish quality-validation
```

Monitor key metrics:
1. **P99 latencies** should remain stable
2. **Error rate** should be < 0.1%
3. **Database connections** should not exceed 80% of max
4. **HTTP connection pool** utilization

## Monitoring Points

### Database Metrics
```sql
-- Check active connections
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';

-- Check connection pool usage
SELECT count(*) FROM pg_stat_activity;
```

### Application Metrics
- `orchestrator_queue_length`: Should stay < 1000
- `http_requests_duration_p99`: Track per endpoint
- `db_pool_connections_active`: Monitor connection usage

## Trade-offs & Considerations

### 1. synchronous_commit=off
- **Benefit**: Significantly faster writes
- **Risk**: Potential data loss on crash (acceptable for trading system with recovery)
- **Mitigation**: All orders are logged; can be recovered from orchestrator queue

### 2. Increased Connection Pools
- **Benefit**: Higher concurrency, fewer waits
- **Cost**: More memory per service
- **Mitigation**: Resource limits prevent unbounded growth

### 3. Shorter HTTP Timeouts
- **Benefit**: Faster failure detection, resource reclamation
- **Risk**: Legitimate slow requests may timeout
- **Mitigation**: Most operations complete in < 100ms; 10s is ample

### 4. Reduced Orchestrator Concurrency
- **Benefit**: Prevents overwhelming downstream services
- **Trade-off**: Slightly slower order processing under extreme load
- **Mitigation**: 300 concurrent tasks still provides excellent throughput

## Rollback Plan

If issues arise, revert in this order:

1. **Immediate**: Increase HTTP timeouts back to 30s
2. **Quick**: Restore synchronous_commit=on for order-db
3. **If needed**: Reduce connection pools to original values
4. **Last resort**: Increase orchestrator concurrency limiter to 1000

## Future Optimizations

If higher throughput needed (1000+ orders/s):

1. **Horizontal Scaling**: Deploy multiple instances of order-service
2. **Database Sharding**: Partition orders by user_id or symbol
3. **Read Replicas**: Offload read queries to replicas
4. **Message Queue**: Replace orchestrator queue with Redis/RabbitMQ
5. **Connection Pooling**: External pgbouncer for better connection management
6. **Caching Layer**: Redis for user balance/holdings lookups

## Conclusion

These optimizations address the core bottlenecks:
- ✅ Database connection exhaustion
- ✅ HTTP client inefficiency  
- ✅ Long timeout cascades
- ✅ PostgreSQL write performance
- ✅ Unbounded concurrency

The system should now sustain 800 orders/s with stable latencies and minimal timeouts.
