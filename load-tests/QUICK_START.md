# Quick Test: 800 Orders/Second Validation

## Run the Test
```fish
cd /home/louis/src/brokerx/load-tests
./run-high-throughput-test.fish
```

## What It Does
- Creates 50 test users with $500k each
- Ramps to 800 orders/second over 2 minutes
- Sustains 800 orders/second for 5 minutes
- Peaks at 1000 orders/second for 2 minutes
- Total duration: ~12 minutes

## Success = All Green ✅
```
✓ http_reqs rate.........................: >800/s
✓ http_req_duration p95..................: <300ms
✓ http_req_failed rate...................: <4.5%
✓ checks.................................: >95%
```

## Quick Check Commands

### Watch live metrics (separate terminal):
```fish
watch -n 1 'curl -s http://localhost:8080/v1/metrics | grep orchestrator'
```

### Check service status:
```fish
docker-compose ps
```

### View service logs:
```fish
docker-compose logs -f order-execution-orchestrator-service
```

## Expected Results
- **Throughput**: 800-1000 orders/s during sustained phase
- **User Service Calls**: ~300-500/s (down from 1400/s before fix)
- **Latency p95**: <250ms
- **Error Rate**: <2%

## If It Fails
1. Check if services are running: `docker-compose ps`
2. Check logs: `docker-compose logs -f`
3. Verify database is healthy
4. Check CPU/memory usage
5. See detailed troubleshooting in `HIGH_THROUGHPUT_TEST.md`

## Performance Fixes Being Tested
✅ User data caching (100ms TTL)
✅ Market price caching (50ms TTL)
✅ HTTP connection pooling (100 per service)
✅ Fast timeouts (500ms instead of 30s)
✅ Concurrency limiting (1000 concurrent orders)

Expected improvement: **10x throughput** (100 → 1000 orders/s)
