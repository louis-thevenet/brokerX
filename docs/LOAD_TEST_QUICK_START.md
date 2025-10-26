# Quick Start Guide - High-Throughput Load Testing

## Prerequisites

1. **System is running:**
   ```bash
   cd /home/louis/src/brokerx
   docker-compose up -d
   ```

2. **Wait for services to be healthy:**
   ```bash
   docker-compose ps
   # All services should show "healthy" status
   ```

## Run the Load Test

```bash
cd /home/louis/src/brokerx/load-tests
./run-tests.fish quality-validation
```

## Monitor During Test

### 1. Real-time Metrics (Grafana)
- **URL:** http://localhost:3000
- **Login:** admin / admin
- **Dashboard:** BrokerX System Metrics

**Key Panels to Watch:**
- Order Creation Rate (should reach 800+ orders/s)
- Request Duration P99 (should stay < 500ms)
- Database Connection Pool Usage (should stay < 80%)
- Error Rate (should stay < 1%)

### 2. Service Logs

```bash
# API Gateway
docker-compose logs -f api-gateway

# Order Service
docker-compose logs -f order-service

# Market Data Service
docker-compose logs -f market-data-service

# User Service
docker-compose logs -f user-service

# Orchestrator
docker-compose logs -f order-execution-orchestrator
```

### 3. System Resources

```bash
# Overall resource usage
docker stats

# Specific service
docker stats market-data-service
```

## Test Results

### Location
Results are saved in: `/home/louis/src/brokerx/load-tests/results/`

Format: `quality-validation-YYYYMMDD-HHMMSS-summary.json`

### Key Metrics to Check

```bash
# View latest summary
cd /home/louis/src/brokerx/load-tests/results
cat (ls -t quality-validation-*-summary.json | head -1) | jq .
```

**Critical Metrics:**

1. **Throughput (orders/second):**
   ```json
   {
     "metrics": {
       "http_reqs": {
         "rate": 850.5  // Should be > 800
       }
     }
   }
   ```

2. **P99 Latency (milliseconds):**
   ```json
   {
     "metrics": {
       "http_req_duration": {
         "values": {
           "p(99)": 450.2  // Should be < 500
         }
       }
     }
   }
   ```

3. **Error Rate (%):**
   ```json
   {
     "metrics": {
       "http_req_failed": {
         "values": {
           "rate": 0.002  // Should be < 0.01 (1%)
         }
       }
     }
   }
   ```

## Success Criteria

✅ **PASS** if:
- Order creation rate ≥ 800 orders/second
- P99 latency < 500ms
- Error rate < 1%
- No service crashes or restarts

❌ **FAIL** if:
- Throughput < 800 orders/second
- P99 latency > 500ms
- Error rate > 1%
- Services crash or become unresponsive

## Troubleshooting

### Low Throughput (< 800 orders/s)

**Check:**
1. Database connection pool usage
   ```bash
   docker-compose logs market-data-service | grep "connection"
   ```

2. CPU/Memory limits
   ```bash
   docker stats
   ```

**Fix:**
- Increase connection pool sizes in `docker-compose.yml`
- Increase CPU limits for bottleneck service
- Check for errors in service logs

### High P99 Latency (> 500ms)

**Check:**
1. Slow queries
   ```bash
   docker-compose exec market-data-db psql -U postgres -d market_data_db -c "SELECT * FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 5;"
   ```

2. Cache hit rate
   ```bash
   docker-compose logs market-data-service | grep "cache"
   ```

**Fix:**
- Increase cache TTL in market-data-service
- Add missing database indexes
- Check for lock contention

### High Error Rate (> 1%)

**Check:**
1. Error types
   ```bash
   docker-compose logs api-gateway | grep "ERROR"
   ```

2. Service health
   ```bash
   docker-compose ps
   curl http://localhost:8080/health
   ```

**Fix:**
- Check service connectivity
- Verify database is accessible
- Check for validation errors in order-service

## Clean Up After Test

### Stop Load Test
```bash
# Press Ctrl+C if still running
```

### Review Results
```bash
cd /home/louis/src/brokerx/load-tests/results
ls -lth quality-validation-*.json | head -5
```

### Clean Up Test Data (Optional)
```bash
# Stop services
docker-compose down

# Remove volumes (WARNING: Deletes all data)
docker-compose down -v

# Restart fresh
docker-compose up -d
```

## Quick Reference - Expected Performance

| Metric | Before Optimization | After Optimization | Target |
|--------|-------------------|-------------------|--------|
| Throughput | 300-400 orders/s | 800+ orders/s | ≥ 800 |
| P99 Latency | 800-1200ms | < 500ms | < 500ms |
| Error Rate | 2-5% | < 1% | < 1% |
| DB Pool Usage | 95-100% | 60-80% | < 80% |
| Cache Hit Rate | N/A | > 80% | > 80% |

## Advanced Monitoring

### Prometheus Queries

Access: http://localhost:9090

**Order Rate:**
```promql
rate(http_requests_total{endpoint="/v1/orders"}[1m])
```

**P99 Latency:**
```promql
histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[1m]))
```

**Error Rate:**
```promql
rate(http_requests_total{status=~"5.."}[1m]) / rate(http_requests_total[1m])
```

### Database Metrics

```bash
# Connection count
docker-compose exec order-db psql -U postgres -d order_db -c "SELECT count(*) FROM pg_stat_activity;"

# Long-running queries
docker-compose exec order-db psql -U postgres -d order_db -c "SELECT pid, now() - query_start as duration, query FROM pg_stat_activity WHERE state = 'active' ORDER BY duration DESC;"

# Cache hit ratio
docker-compose exec market-data-db psql -U postgres -d market_data_db -c "SELECT sum(heap_blks_read) as heap_read, sum(heap_blks_hit) as heap_hit, sum(heap_blks_hit) / (sum(heap_blks_hit) + sum(heap_blks_read)) as ratio FROM pg_statio_user_tables;"
```

## Next Steps After Successful Test

1. **Document Results:**
   - Save summary JSON
   - Take screenshots of Grafana dashboards
   - Note any anomalies or warnings

2. **Production Readiness:**
   - Review `docs/COMPLETE_OPTIMIZATION_SUMMARY.md`
   - Consider additional optimizations from "Future Optimizations" section
   - Plan for monitoring and alerting in production

3. **Continuous Monitoring:**
   - Set up automated load tests
   - Configure alerting thresholds
   - Establish baseline performance metrics
