# WebApp Order API Troubleshooting

## Issue: Place Order Returns JSON Parse Error

### Error Message
```
Network error occurred
Order placement error: SyntaxError: JSON.parse: unexpected character at line 1 column 1 of the JSON data
```

### Root Cause Analysis

The error "JSON.parse: unexpected character at line 1 column 1" typically means:
1. The server returned HTML instead of JSON (likely an error page)
2. The server returned an empty response
3. The API endpoint doesn't exist or isn't running

### Fix Applied

Added the `price` field to the order request payload. The API Gateway's `CreateOrderRequest` includes a separate `price: Option<f64>` field in addition to the `order_type` tagged enum.

**Before:**
```javascript
const orderData = {
  symbol,
  quantity,
  order_side,
  order_type
};
```

**After:**
```javascript
const orderData = {
  symbol,
  quantity,
  order_side,
  order_type,
  price: order_type_value === 'LIMIT' ? parseFloat(price) : null
};
```

### Debugging Steps

1. **Check if API Gateway is running:**
   ```bash
   curl http://localhost:8080/health
   ```
   Expected response:
   ```json
   {
     "status": "healthy",
     "service": "api-gateway",
     "timestamp": "2025-10-13T...",
     "version": "0.1.0"
   }
   ```

2. **Check authentication:**
   - Open browser DevTools → Application → Local Storage
   - Verify `brokerx_token` exists
   - Check the Network tab to see if Authorization header is being sent

3. **Test order creation directly:**
   ```bash
   # Get a token first (login)
   TOKEN="your-jwt-token-here"
   
   # Try to create an order
   curl -X POST http://localhost:8080/orders \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "symbol": "AAPL",
       "quantity": 10,
       "order_side": "Buy",
       "order_type": {"type": "market"},
       "price": null
     }'
   ```

4. **Check browser console for detailed error:**
   - Open browser DevTools → Console
   - Look for the full error stack trace
   - Check Network tab → select the failed request → Preview/Response tab

### Common Issues

#### Issue 1: API Gateway Not Running
**Symptom:** Connection refused or CORS error  
**Solution:** Start the API Gateway:
```bash
cd /home/louis/src/brokerx
cargo run --bin api_gateway
```

#### Issue 2: Invalid Token
**Symptom:** 401 Unauthorized  
**Solution:** Log out and log in again to get a fresh token

#### Issue 3: Order Service Not Running
**Symptom:** API Gateway returns 503 Service Unavailable  
**Solution:** Start the Order Service:
```bash
cd /home/louis/src/brokerx
cargo run --bin order-service
```

#### Issue 4: User Service Not Running
**Symptom:** API Gateway can't validate user, returns 503  
**Solution:** Start the User Service:
```bash
cd /home/louis/src/brokerx
cargo run --bin user-service
```

#### Issue 5: Database Not Running
**Symptom:** Services fail to start or return database errors  
**Solution:** Start PostgreSQL databases:
```bash
cd /home/louis/src/brokerx
./start-databases.sh
```

### Expected API Request/Response

**Request:**
```http
POST /orders HTTP/1.1
Host: localhost:8080
Authorization: Bearer eyJ...
Content-Type: application/json

{
  "symbol": "AAPL",
  "quantity": 10,
  "order_side": "Buy",
  "order_type": {
    "type": "market"
  },
  "price": null
}
```

**Response (201 Created):**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "client_id": "987fcdeb-51d3-4321-b987-123456789abc",
  "date": "2025-10-13T14:30:00Z",
  "symbol": "AAPL",
  "quantity": 10,
  "status": {
    "status": "pending"
  },
  "order_type": {
    "type": "market"
  },
  "order_side": "Buy"
}
```

### Verification Checklist

- [ ] API Gateway running on port 8080
- [ ] Order Service running on port 8082
- [ ] User Service running on port 8081
- [ ] PostgreSQL databases running
- [ ] User logged in with valid JWT token
- [ ] Browser console shows no CORS errors
- [ ] Network tab shows 201 response for order creation
- [ ] Order appears in orders list after creation

### Additional Notes

The `price` field in `CreateOrderRequest` is marked as `Option<f64>` and the API Gateway validates that it's required for limit orders. While the `OrderType` enum also contains the price in the `Limit` variant, the API Gateway model has both for flexibility and validation purposes.
