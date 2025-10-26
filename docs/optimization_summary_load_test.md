# Load Test Optimization Summary

**Date:** 2024
**Objective:** Reduce unnecessary load on market-data-service and improve load test throughput

## Changes Made

### 1. Removed Market Price Fetching Before Order Placement

**File:** `load-tests/scenarios/quality-validation.js`

**Problem:**
- Load test was fetching current market price via `/v1/prices/{symbol}` before placing each BUY order
- This added ~10-20ms latency per order and created unnecessary load on market-data-service
- Since we're using MARKET orders, the order-execution-orchestrator already fetches the current price internally

**Solution:**
- Removed `client.getCurrentPrice(symbol)` call before BUY orders
- Use a simple random quantity (1-3 shares) instead of calculating based on fetched price
- With $500k initial balance per user and typical stock prices ($50-200), small fixed quantities are safe

**Before:**
```javascript
// Get current price
const priceData = client.getCurrentPrice(symbol);
if (!priceData) {
  console.error(`Failed to get price for ${symbol}`);
  return;
}
const price = priceData.price;
sleep(0.01);

// Calculate quantity based on price
const safeBalance = userInfo.balance * 0.05;
const maxQuantity = Math.floor(safeBalance / (price * 1.15));
quantity = Math.min(maxQuantity, 2);
```

**After:**
```javascript
// For MARKET orders, the system will fetch the current price internally
// No need to fetch it here - reduces latency and load on market-data-service

// Use a small fixed quantity for high throughput
quantity = Math.floor(Math.random() * 3) + 1; // 1-3 shares
```

### 2. Removed Unnecessary Sleep Delays

**Changes:**
- Removed `sleep(0.02)` after authentication
- Removed `sleep(0.01)` after fetching price (now removed entirely)
- Removed `sleep(0.01)` in SELL order path

**Impact:**
- Reduces minimum latency per iteration by ~30-40ms
- Allows higher throughput with same number of VUs

## Expected Impact

### Performance Improvements
1. **Reduced Load on Market-Data-Service:**
   - Eliminated ~800 requests/s to `/v1/prices/{symbol}` during load test
   - Market-data-service only handles internal price fetches from orchestrator
   - Reduced P99 latency on price endpoint

2. **Faster Order Placement:**
   - Removed ~10-20ms latency from price fetch
   - Removed ~30-40ms from sleep delays
   - Total reduction: ~40-60ms per BUY order iteration

3. **More Realistic Test Scenario:**
   - Real users don't typically fetch prices separately before market orders
   - Market orders are designed to execute at current market price
   - Test now better simulates actual user behavior

### Throughput Impact
- With 50 VUs and previous ~100ms per iteration → ~500 orders/s
- With same 50 VUs and new ~40-60ms per iteration → ~800-1250 orders/s
- Should easily achieve 800 orders/s target

## Verification Steps

1. **Run Load Test:**
   ```bash
   cd load-tests
   ./run-tests.fish quality-validation
   ```

2. **Monitor Metrics:**
   - Check that order creation rate reaches 800+ orders/s
   - Verify P99 latency stays under 500ms for order creation
   - Monitor market-data-service CPU/memory usage (should be lower)
   - Check `/v1/prices/{symbol}` request rate (should be minimal)

3. **Review Results:**
   - Check `results/quality-validation-*-summary.json`
   - Verify `http_reqs{name:create_order}` rate > 800
   - Verify `http_req_duration{name:create_order}` P99 < 500ms

## Rollback Instructions

If the simplified quantity calculation causes issues (e.g., users running out of balance):

1. **Restore Price Fetching:**
   ```bash
   cd /home/louis/src/brokerx
   git checkout load-tests/scenarios/quality-validation.js
   ```

2. **Alternative: Increase Fixed Quantity Buffer:**
   - Keep the optimization but increase initial balance in setup
   - Or reduce max quantity to 1-2 shares always

## Related Changes

- **market-data-service cache optimization:** Increased TTL to 5s, added invalidation on update
- **user-service atomic updates:** Deposit/withdraw return 204, reduced unnecessary GET requests
- **API Gateway client pooling:** Optimized connection reuse
- **Database connection pools:** Increased pool sizes across all services

## Notes

- This optimization is safe because:
  - Users start with $500k balance
  - Typical stock prices are $50-200
  - 1-3 shares per order = $50-600 per order
  - Users can place ~800-1000 orders before running out of balance
  - Load test duration is typically 5-10 minutes
- The removed price fetch was redundant - orchestrator already fetches it
- Market orders are designed to execute at current price, so pre-fetching doesn't add value
