# Market Price Caching Architecture

## Implementation: API Gateway Price Cache

We've implemented **Option 1**: Cache market prices in the API Gateway.

### Architecture

```
┌─────────────────┐
│   API Gateway   │
│                 │
│  ┌───────────┐  │
│  │  Price    │  │  100ms TTL
│  │  Cache    │  │  1000 symbols max
│  └───────────┘  │
│        ↓        │
│  ┌───────────┐  │
│  │  Market   │  │  Only on cache miss
│  │  Data API │  │
│  └───────────┘  │
└─────────────────┘
         ↑
         │ Get price (cached)
         │
┌────────────────────┐
│   Orchestrator     │
│   Service          │
└────────────────────┘
```

### Why This Approach?

✅ **Lower Latency**
- In-process cache lookup (< 1ms)
- No HTTP overhead
- No network round-trip

✅ **Reduced Load**
- Market data service called only on cache miss
- Expected 80-90% cache hit rate
- Reduces market data calls from ~800/s to ~100/s

✅ **Single Source of Truth**
- All clients get same cached price
- Consistent pricing across requests
- Simpler to manage

✅ **Better Scalability**
- API Gateway already handles all incoming traffic
- Cache scales with API Gateway instances
- Less pressure on market data service

### Configuration

**Cache Settings** (`api_gateway/src/main.rs`):
```rust
let price_cache = Cache::builder()
    .max_capacity(1_000)  // Up to 1000 symbols
    .time_to_live(Duration::from_millis(100))  // 100ms TTL
    .build();
```

**Why 100ms TTL?**
- Market prices don't change every millisecond
- 100ms is fresh enough for order validation
- High enough to get good cache hit rates
- Low enough to reflect recent price movements

### Usage

The orchestrator service can now call the API Gateway's price endpoint to get cached prices:

**Before** (calling market-data directly):
```
Orchestrator → Market Data Service (every time)
```

**After** (using API Gateway cache):
```
Orchestrator → API Gateway → Price Cache (fast!)
                          ↓
                  Market Data Service (only on cache miss)
```

### Expected Performance Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Price lookup latency | 5-20ms | <1ms | 5-20x faster |
| Market data RPS | 800/s | ~100/s | 80% reduction |
| Network calls | Every request | Cache miss only | 80-90% fewer |
| Orders/second capacity | 750/s | **800+/s** | **7%+ increase** |

### Cache Hit Rate Expectations

With 8 trading symbols and 800 orders/s:
- **Without cache**: 800 price lookups/s to market data
- **With cache (100ms TTL)**: 
  - Each symbol cached for 100ms
  - 10 refreshes/second per symbol
  - ~80 cache misses/s total
  - **Cache hit rate: 90%**

### Monitoring

Monitor these metrics to validate the cache is working:

```prometheus
# Cache statistics (if instrumented)
price_cache_hits / (price_cache_hits + price_cache_misses)  # Should be >80%

# Market data service RPS (should drop significantly)
rate(http_requests_total{service="market-data"}[1m])

# API Gateway price endpoint latency (should be <1ms cached)
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{endpoint="/v1/market/prices/:symbol"}[1m]))
```

### Code Changes

1. **API Gateway** (`api_gateway/`):
   - Added `moka` dependency for caching
   - Added `price_cache` to `AppState`
   - Modified `get_market_price()` to check cache first
   - Cache automatically expires after 100ms

2. **Orchestrator** (recommended change):
   - Update trade validation to call API Gateway instead of market-data service
   - Reuse existing HTTP client pool
   - Benefit from cached prices

### Alternative Considered: Cache in Market Data Service

❌ **Why we didn't choose this**:
- Still requires HTTP call from orchestrator
- Network latency remains
- Connection pool contention
- Doesn't address the root cause (HTTP overhead)
- Less flexible (cache doesn't help other services)

### Next Steps

To fully utilize this cache:

1. **Update Orchestrator Trade Validation**:
   - Modify `trade_validation.rs` to call API Gateway's price endpoint
   - Remove direct market-data service dependency for prices
   - Keep execution calls to market-data service (different use case)

2. **Monitor Cache Effectiveness**:
   - Add metrics for cache hits/misses
   - Monitor market data service load reduction
   - Validate latency improvements

3. **Tune TTL if Needed**:
   - If prices are too stale: reduce TTL to 50ms
   - If cache hit rate is low: increase TTL to 200ms
   - Monitor and adjust based on real traffic

### Files Modified

- `api_gateway/Cargo.toml` - Added `moka` dependency
- `api_gateway/src/main.rs` - Added price cache to AppState
- `api_gateway/src/api.rs` - Modified `get_market_price()` to use cache

### Performance Testing

Run the high-throughput test to validate:

```fish
cd load-tests
./run-high-throughput-test.fish
```

Expected results:
- ✅ 800+ orders/second sustained
- ✅ <1ms price lookup latency (p95)
- ✅ 80%+ cache hit rate
- ✅ 80% reduction in market-data service calls
