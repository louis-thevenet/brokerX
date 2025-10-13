# WebApp Order Placement Fix

**Date:** 2025-10-13  
**Issue:** Order placement not working - "JSON parse error"  
**Status:** ✅ Fixed

## Root Cause

The webapp was sending incorrect values for the `order_side` field:
- **Webapp was sending:** `"BUY"` and `"SELL"` (all uppercase)
- **API expects:** `"buy"` and `"sell"` (all lowercase)

The API Gateway's `OrderSide` enum is defined with:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}
```

The `#[serde(rename_all = "lowercase")]` attribute means the enum expects lowercase values.

## Fixes Applied

### 1. JavaScript - Convert order_side to lowercase
**File:** `/home/louis/src/brokerx/webapp/app.js`

```javascript
// Before
const order_side = document.getElementById('order-side').value;

// After
const order_side = document.getElementById('order-side').value.toLowerCase();
```

### 2. HTML - Update form options to lowercase
**File:** `/home/louis/src/brokerx/webapp/index.html`

```html
<!-- Before -->
<option value="BUY">Buy</option>
<option value="SELL">Sell</option>

<!-- After -->
<option value="buy">Buy</option>
<option value="sell">Sell</option>
```

### 3. JavaScript - Update quickBuy function
**File:** `/home/louis/src/brokerx/webapp/app.js`

```javascript
// Before
document.getElementById('order-side').value = 'BUY';

// After  
document.getElementById('order-side').value = 'buy';
```

### 4. Display - Capitalize for presentation
**File:** `/home/louis/src/brokerx/webapp/app.js`

Added CSS text-transform to capitalize the order side in the display table:
```javascript
<td style="text-transform: capitalize;">${order.order_side}</td>
```

### 5. Enhanced Error Logging
Added detailed console logging to help debug issues:
```javascript
console.log('Placing order:', orderData);
console.log('Order response status:', response.status);
console.log('Order response headers:', response.headers);
```

Also improved error handling to show the actual response text when JSON parsing fails.

## API Contract

The correct order creation request format is:

```json
{
  "symbol": "AAPL",
  "quantity": 10,
  "order_side": "buy",
  "order_type": {
    "type": "market"
  },
  "price": null
}
```

Or for limit orders:

```json
{
  "symbol": "AAPL",
  "quantity": 10,
  "order_side": "sell",
  "order_type": {
    "type": "limit",
    "price": 150.50
  },
  "price": 150.50
}
```

## Testing

After these fixes, test the following scenarios:

1. **Market Buy Order:**
   - Select symbol (e.g., AAPL)
   - Enter quantity
   - Select "Buy" side
   - Select "Market" type
   - Submit

2. **Market Sell Order:**
   - Select symbol
   - Enter quantity
   - Select "Sell" side
   - Select "Market" type
   - Submit

3. **Limit Buy Order:**
   - Select symbol
   - Enter quantity
   - Select "Buy" side
   - Select "Limit" type
   - Enter price
   - Submit

4. **Limit Sell Order:**
   - Select symbol
   - Enter quantity
   - Select "Sell" side
   - Select "Limit" type
   - Enter price
   - Submit

5. **Quick Buy from Market Data:**
   - Click "📈 Buy" button on any stock in market data table
   - Verify form is pre-filled with lowercase "buy"
   - Enter quantity and submit

## Expected Behavior

After the fix:
- ✅ Orders submit successfully
- ✅ Status code 201 (Created) returned
- ✅ Order appears in orders list
- ✅ Success message displayed
- ✅ Balance updated after order placement
- ✅ Order side displays as "Buy" or "Sell" (capitalized for readability)

## Files Modified

1. `/home/louis/src/brokerx/webapp/app.js`
   - Added `.toLowerCase()` to order_side
   - Updated quickBuy to use lowercase
   - Added text-transform CSS for display
   - Enhanced error logging

2. `/home/louis/src/brokerx/webapp/index.html`
   - Changed option values from "BUY"/"SELL" to "buy"/"sell"

## Additional Notes

- The `OrderType` enum also uses `#[serde(rename_all = "lowercase")]`, so "market" and "limit" must be lowercase
- The webapp already sends these correctly: `{ type: "market" }` or `{ type: "limit", price: 150.0 }`
- The `price` field is sent as a separate field AND embedded in the `order_type` for limit orders

## Related Issues

- Initial issue: "JSON parse error" was misleading - the actual problem was a deserialization error
- The API Gateway was likely returning a 400 Bad Request with an error message, but the webapp was trying to parse it as the success response
- Enhanced error logging now shows the actual error response

## Verification

To verify the fix is working:

1. Open browser DevTools → Console
2. Place an order
3. Check console logs:
   ```
   Placing order: {symbol: "AAPL", quantity: 10, order_side: "buy", ...}
   Order response status: 201
   Order created: {id: "...", ...}
   ```

4. Check Network tab → POST /orders → Response should be 201 with order data
