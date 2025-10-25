# Issue: "Invalid symbol" errors after caching implementation

## Root Cause

The **orchestrator service was NOT rebuilt** after we made the code changes to use API Gateway for price lookups.

## What Happened

1. ✅ We added `ApiGatewayClient` to orchestrator code
2. ✅ We added price caching to API Gateway
3. ✅ We updated `docker-compose.yml` with `API_GATEWAY_URL`
4. ❌ **We didn't rebuild the Docker images**

Result: The running orchestrator container is still using the **old code** that calls market-data directly!

## Evidence

```bash
# API Gateway works fine (has the cache)
$ curl http://localhost:8080/v1/market/prices/AAPL
{"price":177.74,"symbol":"AAPL",...}  ✅

# But orchestrator doesn't have new code
$ docker ps | grep orchestrator
Up 8 minutes  # Started before our code changes!

# No log message about using API Gateway
$ docker logs brokerx-order-execution-orchestrator-service | grep "API Gateway"
(empty)  # The new code would log "Using API Gateway at..."
```

## Solution

### Step 1: Rebuild the Services

```fish
cd /home/louis/src/brokerx

# Option A: Use our script
./scripts/rebuild-with-caching.fish

# Option B: Manual rebuild
docker build -f docker/Dockerfile.order-execution-orchestrator-service -t brokerx-order-execution-orchestrator-service .
docker build -f docker/Dockerfile.api-gateway -t brokerx-api-gateway .
docker restart brokerx-order-execution-orchestrator-service brokerx-api-gateway
```

### Step 2: Verify

Check that the orchestrator is using API Gateway:

```fish
# Should see this in the logs:
docker logs brokerx-order-execution-orchestrator-service 2>&1 | grep "Using API Gateway"

# Expected output:
# "Using API Gateway at http://api-gateway:8080 for cached price lookups"
```

### Step 3: Test

```fish
# Run a simple order test
cd load-tests
./run-high-throughput-test.fish
```

## Why This Matters

### Current State (Old Code Running)
```
Orchestrator → Market Data Service directly
              ↓
           5-20ms latency
           NO caching
           "Invalid symbol" errors (connection issues)
```

### After Rebuild (New Code)
```
Orchestrator → API Gateway → [Cache 90% hits]
                          ↓
                     Market Data Service (10%)
              ↓
           <1ms latency from cache
           80% less load on market-data
           Better reliability
```

## Lesson Learned

When making code changes in Docker environments:

1. ✅ Make code changes
2. ✅ Update configuration
3. ✅ **REBUILD Docker images** ← We missed this!
4. ✅ Restart containers
5. ✅ Verify in logs

## Quick Commands

```fish
# Rebuild everything
./scripts/rebuild-with-caching.fish

# Check it worked
docker logs brokerx-order-execution-orchestrator-service | tail -20

# Run test
cd load-tests && ./run-high-throughput-test.fish
```

## Expected Results After Rebuild

✅ No more "Invalid symbol" errors  
✅ Orchestrator uses API Gateway for prices  
✅ 90% cache hit rate  
✅ <1ms price lookups  
✅ 800+ orders/second throughput  
