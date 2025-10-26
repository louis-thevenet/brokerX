# Cache Stampede & Latency Spike Fixes

**Date**: 2024
**Objective**: Eliminate P99 latency spikes in `/v1/prices/{symbol}` endpoint by fixing cache stampede and optimizing caching strategy

## Problem Analysis

### Observed Issues
1. **P99 latency spikes up to 2+ seconds** on `/v1/prices/{symbol}` endpoint
2. **Intermittent performance degradation** under 800 req/s load
3. **Cache stampede**: Multiple concurrent requests hitting DB when cache expires
4. **Double caching overhead**: API Gateway (2.5s TTL) + Market-data-service (5s TTL)

### Root Causes
1. **Inefficient two-layer caching**:
   - API Gateway cached prices with 2.5s TTL using `moka::Cache`
   - Market-data-service cached with 5s TTL
   - When API Gateway cache expired, all concurrent requests would hit market-data-service
   - Extra serialization overhead from using `serde_json::Value` in cache

2. **No request coalescing**:
   - API Gateway used `try_get_with` but only for its own cache
   - Market-data-service did simple get/insert pattern with race conditions
   - When cache missed, multiple concurrent requests would all query DB

3. **Verbose logging in hot path**:
   - `info!` and `debug!` logs on every price fetch
   - String formatting overhead even when logs not output

4. **Long HTTP timeouts**:
   - 10s timeout allowed slow requests to pile up
   - Connection pool could get exhausted waiting for slow responses

## Solutions Implemented

### 1. Eliminated Double Caching
**Files**: `api_gateway/src/api.rs`, `api_gateway/src/main.rs`

- **Removed** API Gateway price cache entirely
- Fetch directly from market-data-service (which has optimized caching)
- Simplified architecture: one cache layer instead of two
- Removed unused `moka::Cache` import and `price_cache` field from `AppState`

```rust
// Before: Double caching
let price = state.price_cache.try_get_with(symbol, async {
    state.market_data_client.get_price(&symbol).await
}).await?;

// After: Direct fetch (market-data-service has cache)
let price = state.market_data_client.get_price(&symbol).await?;
```

### 2. Request Coalescing in Market-Data-Service
**File**: `market-data-service/src/repository.rs`

- **Implemented** `try_get_with` pattern in repository layer
- When multiple requests for same symbol arrive concurrently:
  - Only **ONE** queries the database
  - Others **wait** and share the result
- Prevents cache stampede at DB level
- Increased cache TTL from 5s to **10s** for better hit rate

```rust
// Before: Simple cache check with race condition
if let Some(cached) = self.price_cache.get(symbol).await {
    return Ok((*cached).clone());
}
let price = query_db(&self.pool).await?;
self.price_cache.insert(symbol, Arc::new(price.clone())).await;

// After: Request coalescing
let price = self.price_cache.try_get_with(symbol.to_string(), async {
    let price = query_db(&self.pool).await?;
    Ok::<_, RepositoryError>(Arc::new(price))
}).await?;
```

### 3. Removed Verbose Logging
**File**: `api_gateway/src/clients.rs`, `api_gateway/src/api.rs`

- **Removed** `info!` logs from hot paths (price fetching)
- **Removed** `debug!` logs for routine operations
- **Kept** `error!` logs for failure cases
- Reduced string formatting overhead

### 4. Reduced HTTP Client Timeouts
**File**: `api_gateway/src/clients.rs`

- **Timeout**: 10s → **5s** (fail faster)
- **Connect timeout**: 3s → **2s**
- Prevents request pileup when services slow down
- Allows faster failure detection and retry at load balancer level

## Expected Performance Improvements

### Latency Reduction
- **P99 latency**: Expected reduction from 2s spikes to <500ms
- **Cache hit rate**: Improved from ~80% to >95% with 10s TTL
- **DB load**: Reduced by 80-90% due to request coalescing

### Throughput Stability
- **Sustained load**: Should handle 800+ req/s consistently
- **No cache stampede**: Only 1 DB query per symbol per 10s window
- **Faster failure recovery**: 5s timeout prevents cascading delays

### Resource Efficiency
- **Memory**: Reduced overhead from eliminating duplicate cache
- **CPU**: Reduced from less logging and serialization
- **DB connections**: Better utilization due to less concurrent queries

## Testing & Validation

### Performance Tests
Run high-throughput load test and monitor:
```bash
cd load-tests
./run-high-throughput-test.fish
```

### Metrics to Monitor
1. **P99 latency** on `/v1/prices/{symbol}`: Target <500ms
2. **Cache hit rate**: Target >95%
3. **DB connection usage**: Should stay <50% of pool
4. **Error rate**: Should stay <0.1%
5. **Request coalescing effectiveness**: Monitor duplicate DB queries

### Success Criteria
- [ ] P99 latency consistently <500ms at 800 req/s
- [ ] No timeout errors under sustained load
- [ ] Cache hit rate >95%
- [ ] DB query rate <10 per symbol per minute

## Monitoring & Observability

### Key Metrics
- `http_request_duration_seconds{endpoint="/v1/prices/{symbol}"}` - P50, P95, P99
- Market-data-service cache stats (if instrumented)
- Database query execution time
- HTTP client timeout/error rates

### Debugging Tools
If issues persist:
1. Enable query logging in PostgreSQL to see DB load
2. Add cache hit/miss metrics to market-data-service
3. Monitor connection pool exhaustion
4. Check for slow query patterns

## Rollback Plan

If performance degrades:
1. Revert `api_gateway/src/api.rs` to restore double caching
2. Revert `market-data-service/src/repository.rs` to 5s TTL
3. Increase timeouts back to 10s if needed
4. Re-enable logging for debugging

## Next Steps

### If Spikes Persist
1. **Add pgbouncer**: Connection pooler to further reduce DB load
2. **Increase cache TTL**: Consider 15-30s if data staleness acceptable
3. **Pre-warm cache**: Background job to keep hot symbols cached
4. **Horizontal scaling**: Add more market-data-service replicas
5. **Database read replicas**: Distribute query load

### If Spikes Eliminated
1. Monitor for 24 hours under production load
2. Document baseline performance metrics
3. Consider gradual cache TTL increase (10s → 15s → 20s)
4. Remove old documentation about double caching

## Related Documentation
- `PERFORMANCE_OPTIMIZATIONS.md` - Previous optimization round
- `MARKET_DATA_OPTIMIZATIONS.md` - DB-level optimizations
- `FINAL_OPTIMIZATIONS_SUMMARY.md` - Complete optimization history
