# High-Throughput Order Processing Test

## Overview
This test validates that the order-execution-orchestrator-service can handle **800 orders/second sustained** with a peak of **1000 orders/second**, following the performance fixes implemented.

## Test Configuration

### Scenario: High-Throughput Quality Validation
- **Type**: Arrival rate-based (requests per second)
- **Duration**: ~12 minutes total
- **Test Users**: 50 users with $500k balance each
- **Order Size**: Small (1-3 shares) to maximize throughput

### Load Profile

| Phase | Duration | Target Rate | Purpose |
|-------|----------|-------------|---------|
| Warm up | 30s | 100 orders/s | System warm-up |
| Ramp up | 30s | 400 orders/s | Gradual increase |
| Target | 1m | 800 orders/s | Reach target |
| **Sustained** | **5m** | **800 orders/s** | **Main test phase** |
| Push | 1m | 1000 orders/s | Test peak capacity |
| Peak | 2m | 1000 orders/s | Sustained peak |
| Ramp down | 1m | 400 orders/s | Gradual decrease |
| Cool down | 30s | 0 orders/s | System recovery |

**Total**: ~12 minutes, with 5 minutes at target 800 orders/s

## Success Criteria

### Performance Thresholds

| Metric | Threshold | Target |
|--------|-----------|--------|
| **Throughput** | >800 orders/s | 800-1000 orders/s |
| **Latency (p95)** | <300ms | <250ms |
| **Latency (p99)** | <600ms | <500ms |
| **Error Rate** | <4.5% | <2% |
| **Success Rate** | >95.5% | >98% |

### Expected Improvements
Based on the performance fixes:
- **Before**: ~100 orders/second
- **After**: 800-1000 orders/second
- **Improvement**: 8-10x throughput increase

## Running the Test

### Prerequisites
1. **k6** installed (`brew install k6` on macOS)
2. All services running (`docker-compose up`)
3. Database initialized with test symbols

### Quick Start
```fish
cd load-tests
./run-high-throughput-test.fish
```

### Manual Run
```fish
cd load-tests
k6 run --out json=results/output.json scenarios/quality-validation.js
```

### Environment Variables
```fish
# Override API Gateway URL
set -x BASE_URL "http://localhost:8080"

# Run the test
./run-high-throughput-test.fish
```

## Test Optimizations

### For High Throughput
The test is optimized to minimize client-side overhead:

1. **More Users (50)**: Reduces contention on individual user accounts
2. **Higher Balances ($500k)**: Ensures users don't run out of funds
3. **Smaller Orders (1-3 shares)**: Faster processing, more orders
4. **Minimal Delays**: 0.02-0.05s sleeps instead of 0.1-0.5s
5. **Reduced Logging**: Only logs 1% of successful orders
6. **Conservative Sizing**: Only uses 5% of balance per order

## Performance Fixes Validated

This test validates the following optimizations:

### 1. User Data Caching
- **Cache TTL**: 100ms
- **Expected**: 60-70% reduction in user service calls
- **Validation**: User service RPS should be ~300-400 (vs 1400 before)

### 2. Market Price Caching
- **Cache TTL**: 50ms
- **Expected**: 50-80% reduction in price lookups
- **Validation**: Market data price calls should be reduced

### 3. HTTP Client Optimization
- **Connection pooling**: Up to 100 connections per service
- **Fast timeouts**: 500ms (vs 30s before)
- **Expected**: Faster failure detection, better connection reuse

### 4. Concurrency Control
- **Semaphore limit**: 1000 concurrent orders
- **Expected**: Prevents service overload while maintaining throughput

## Interpreting Results

### Success Indicators
✅ **Passed**: 800+ orders/s sustained for 5 minutes
✅ **Excellent**: 900+ orders/s sustained
✅ **Outstanding**: 1000+ orders/s sustained

### Metrics to Check

#### Throughput
```
http_reqs................................: 800/s (target: >800)
http_reqs{name:create_order}..............: 800/s (target: >800)
```

#### Latency
```
http_req_duration.........................: p95=250ms p99=500ms
http_req_duration{name:create_order}......: p95=250ms p99=500ms
```

#### Errors
```
http_req_failed...........................: 2% (target: <4.5%)
checks....................................: 98% (target: >95%)
```

### Common Issues

#### Low Throughput (<800 orders/s)
- **Cause**: Downstream service bottleneck
- **Check**: Service logs, CPU, database connections
- **Fix**: Scale services, optimize queries

#### High Latency (p95 >300ms)
- **Cause**: Slow database or external services
- **Check**: Service response times, cache hit rates
- **Fix**: Add indexes, increase cache TTL

#### High Error Rate (>5%)
- **Cause**: Validation failures or service errors
- **Check**: Error logs, validation rules
- **Fix**: Adjust test data, fix validation logic

## Monitoring During Test

### Real-time Metrics (Prometheus)
```
# Order processing rate
rate(orchestrator_orders_processed_total[1m])

# Queue depth
orchestrator_queue_length

# Cache hit ratio (if instrumented)
user_cache_hits / (user_cache_hits + user_cache_misses)
```

### Service Health
1. **CPU Usage**: Should stay <80% on all services
2. **Memory**: Should be stable, no leaks
3. **Database Connections**: Should not exhaust pool
4. **HTTP Connections**: Should stabilize with pooling

## Next Steps

### After Successful Test
1. Run soak test (30min+ sustained load)
2. Run stress test (push to failure point)
3. Monitor production metrics
4. Set up alerts for throughput drops

### If Test Fails
1. Check service logs for errors
2. Monitor resource usage (CPU, memory, connections)
3. Verify cache is working (check hit rates)
4. Run smaller load test to isolate issue
5. Check database performance

## Files
- **Test Script**: `scenarios/quality-validation.js`
- **Config**: `config.js` (see `scenarios.highThroughput`)
- **Runner**: `run-high-throughput-test.fish`
- **Results**: `results/high-throughput-*.json`

## Related Documentation
- [Performance Fix Details](../docs/PERFORMANCE_FIX.md)
- [Order Orchestrator Implementation](../docs/order-orchestrator-implementation.md)
- [Load Testing Guide](./README.md)
