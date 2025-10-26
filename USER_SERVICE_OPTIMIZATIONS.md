# User Service Deposit/Withdraw Optimization

## Problem Identified

Deposit and withdraw endpoints were causing **3 database queries per operation**:

### Before Optimization:
```
1. Repository: SELECT user (get_user)
2. Repository: UPDATE user (set new balance)
3. API: SELECT user again (to return updated user)
```

At 800 orders/s with financial transactions, this created massive DB load.

## Solution Applied

### 1. Repository Layer - Single UPDATE Query ✅

**File:** `user-service/src/repository.rs`

Changed from SELECT + UPDATE to single atomic UPDATE:

```rust
// Before: 2 queries
let user = get_user(id)?;      // SELECT
user.balance += amount;
update_user(user)?;             // UPDATE

// After: 1 query
sqlx::query!(
    "UPDATE users SET balance = balance + $1 WHERE id = $2",
    amount, user_id
).execute()?;
```

**Impact:** 66% reduction in DB queries (3 → 1)

### 2. API Layer - Return Simple Success ✅

**File:** `user-service/src/api.rs`

Changed endpoints to return `204 No Content` instead of full user object:

```rust
// Before
pub async fn deposit_funds(...) -> Json<UserResponse> {
    service.deposit(user_id, amount).await?;
    let user = service.get_user(user_id).await?;  // Extra SELECT!
    Ok(Json(user))
}

// After  
pub async fn deposit_funds(...) -> StatusCode {
    service.deposit(user_id, amount).await?;
    Ok(StatusCode::NO_CONTENT)  // No extra SELECT
}
```

### 3. Client Layer - Fetch User Only When Needed ✅

**File:** `api_gateway/src/clients.rs`

API Gateway now does explicit GET only when user data is needed:

```rust
pub async fn deposit_funds(&self, user_id: Uuid, amount: f64) -> ApiResult<UserResponse> {
    // Step 1: Fast deposit (1 DB query in user-service)
    self.client.post(...).send().await?;
    
    // Step 2: Fetch user only if caller needs it (1 DB query)
    self.get_user(user_id).await
}
```

**For Orchestrator:** Already returns `()`, so it skips step 2 entirely!

## Performance Improvement

### Orchestrator Flow (High Volume - 800 ops/s)

**Before:**
- 3 DB queries per deposit/withdraw
- At 800 ops/s: 2,400 queries/second

**After:**
- 1 DB query per deposit/withdraw  
- At 800 ops/s: 800 queries/second

**Result: 67% reduction in DB load**

### User-Facing API (Low Volume - <10 ops/s)

**Before:**
- 3 DB queries: deposit + get_user in API

**After:**
- 2 DB queries: deposit + explicit get_user in client

**Result: Slight improvement, negligible volume**

## Expected Latency Improvements

| Endpoint | Before P99 | Target P99 | Improvement |
|----------|-----------|-----------|-------------|
| `/v1/users/{id}/deposit` | 227ms | **<20ms** | 91% |
| `/v1/users/{id}/withdraw` | 228ms | **<20ms** | 91% |

## Files Modified

1. ✅ `user-service/src/repository.rs` - Atomic UPDATE queries
2. ✅ `user-service/src/service.rs` - Simplified deposit/withdraw
3. ✅ `user-service/src/api.rs` - Return 204 No Content
4. ✅ `api_gateway/src/clients.rs` - Explicit get_user when needed
5. ⚠️ `order-execution-orchestrator-service/src/clients.rs` - No changes needed (already optimized)

## Database Query Reduction

### Per Operation

| Operation | Before | After | Saved |
|-----------|--------|-------|-------|
| Deposit (orchestrator) | 3 queries | 1 query | 2 queries |
| Withdraw (orchestrator) | 3 queries | 1 query | 2 queries |
| Deposit (user API) | 3 queries | 2 queries | 1 query |
| Withdraw (user API) | 3 queries | 2 queries | 1 query |

### At 800 Orders/Second

Assuming 50% of orders require financial transactions:

- **Operations/second:** 400 deposits + 400 withdraws = 800 ops
- **Queries before:** 800 × 3 = 2,400 queries/s
- **Queries after:** 800 × 1 = 800 queries/s
- **Reduction:** 1,600 fewer queries/second (67%)

## Trade-offs

### ✅ Pros
- Massive DB load reduction
- Faster response times
- Better scalability
- Atomic updates (safer)

### ⚠️ Considerations
- API Gateway makes 2 calls for user-facing deposits (vs 1 before)
- But user-facing volume is <1% of total, so negligible impact
- Orchestrator (99% of volume) only makes 1 call

## Testing

```bash
# Rebuild services
docker-compose build user-service api-gateway

# Restart
docker-compose up -d

# Run load test
cd load-tests
./run-tests.fish quality-validation
```

## Expected Metrics

### Database Connections
- User DB active connections should drop from ~80 to ~40

### Query Rate
```sql
-- Check query rate
SELECT count(*) FROM pg_stat_activity 
WHERE state = 'active' AND query LIKE '%UPDATE users%';
```

Should see ~50-70% reduction in active UPDATE queries.

### Latency
- Deposit P99: 227ms → **<20ms**
- Withdraw P99: 228ms → **<20ms**

## Rollback Plan

If issues arise:

```bash
git checkout user-service/src/repository.rs
git checkout user-service/src/service.rs  
git checkout user-service/src/api.rs
git checkout api_gateway/src/clients.rs
docker-compose build user-service api-gateway
docker-compose up -d
```

## Summary

This optimization specifically targets the high-volume orchestrator flow where user data is NOT needed after deposit/withdraw. By eliminating unnecessary SELECT queries, we reduce DB load by 67% for financial operations, which should eliminate the deposit/withdraw bottleneck at 800 orders/s.

---

**Status:** Ready to test. User service deposit/withdraw should now handle 1000+ ops/s with sub-20ms P99.
