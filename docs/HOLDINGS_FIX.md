# Holdings Display Fix

## Problem
Holdings were not displaying in the webapp, and when they did display, profit/loss calculations showed incorrect values (e.g., -100% return).

## Root Causes

### 1. Missing GET Holdings Endpoint
- The user service had an `update_holdings` (POST) endpoint but no GET endpoint
- The webapp was trying to call `/users/{id}/holdings` which didn't exist
- **Fix**: Added `get_holdings()` endpoint to user service that returns the holdings from the User object

### 2. Holdings Already in User Object
- The User object already contains a `holdings` HashMap field
- The API Gateway's `/me` endpoint returns the full User object with holdings
- **Fix**: Modified webapp to use holdings from `currentUser` object instead of making a separate API call

### 3. Market Data Not Loaded When Displaying Holdings
- `showDashboard()` called `loadMarketData()` and `loadHoldings()` in parallel
- Holdings were displayed before market prices were available
- Profit/loss calculations used `currentPrice = 0` because `marketDataCache` was empty
- **Fix**: Changed to load market data first, then load holdings after prices are available

## Changes Made

### User Service (`user-service/src/api.rs`)
```rust
/// Get user holdings
pub async fn get_holdings(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match state.user_service.get_user(&user_id).await {
        Ok(Some(user)) => {
            let holdings: Vec<_> = user.holdings.values().cloned().collect();
            Json(holdings).into_response()
        }
        // ... error handling
    }
}
```

### User Service Routes (`user-service/src/main.rs`)
```rust
.route("/users/{id}/holdings", get(api::get_holdings))
.route("/users/{id}/holdings", post(api::update_holdings))
```

### Webapp (`webapp/app.js`)

#### 1. Use holdings from currentUser object
```javascript
async function loadHoldings() {
  if (!currentUser || !currentUser.holdings) {
    return;
  }
  
  const holdings = Object.values(currentUser.holdings);
  displayHoldings(holdings);
}
```

#### 2. Load market data before holdings
```javascript
function showDashboard() {
  // Load market data first, then holdings (so prices are available)
  loadMarketData().then(() => {
    loadHoldings();
  });
  loadOrders();
  startMarketDataRefresh();
}
```

#### 3. Added debug logging
```javascript
console.log('Displaying holdings:', holdings);
console.log('Market data cache:', marketDataCache);
console.log(`Holding ${symbol}: quantity=${qty}, current_price=${price}`);
```

## How It Works Now

1. User logs in → API Gateway returns User object with holdings
2. `loadUserProfile()` stores user in `currentUser` (including holdings)
3. `showDashboard()` is called:
   - Loads market data first
   - After market data loads, loads holdings from `currentUser.holdings`
   - Holdings display with correct current prices
   - Profit/loss calculations work correctly

## Holdings Data Flow

```
Order Execution (API Gateway)
  ↓
Update Holdings (User Service)
  ↓
Holdings stored in User.holdings HashMap
  ↓
User logs in / refreshes profile
  ↓
API Gateway /me returns User with holdings
  ↓
Webapp stores in currentUser
  ↓
Holdings displayed with current market prices
```

## Testing

To verify the fix:
1. Place and execute an order
2. Refresh the dashboard
3. Holdings section should show:
   - Correct quantity
   - Average cost (execution price)
   - Current market price
   - Accurate profit/loss
   - Correct return percentage

## Future Improvements

- Add real-time holdings updates via WebSocket
- Cache market data more efficiently
- Add holdings history/transactions view
- Show cost basis details for tax purposes
