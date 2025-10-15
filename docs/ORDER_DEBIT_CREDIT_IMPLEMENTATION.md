# Order Debit/Credit Implementation

## Overview
Added client debit/credit functionality to the order creation orchestration logic. When orders are executed (fully or partially filled), the system now automatically debits or credits the user's account balance based on the order side.

## Changes Made

### 1. User Service (`user-service/`)

#### Added Withdraw Endpoint
- **File**: `src/api.rs`
  - Added `WithdrawRequest` struct for the withdraw endpoint
  - Implemented `withdraw_funds` endpoint handler
  - Returns 400 BAD_REQUEST if insufficient funds
  - Returns updated user with new balance on success

- **File**: `src/service.rs`
  - Added `withdraw_from_user` method to `UserService`
  - Delegates to repository layer's existing `withdraw_from_user` method

- **File**: `src/main.rs`
  - Added route: `POST /users/{id}/withdraw`

### 2. API Gateway (`api_gateway/`)

#### Added Client Methods
- **File**: `src/clients.rs`
  - Added `withdraw_funds` method to `UserClient`
  - Handles withdraw requests to user service
  - Properly handles error cases (not found, insufficient funds, service unavailable)

#### Enhanced Order Creation Orchestration
- **File**: `src/api.rs`
  - Updated `create_order` handler (Step 4) to include debit/credit logic
  - **Buy Orders**: Debits user account by `execution_price * filled_quantity`
  - **Sell Orders**: Credits user account by `execution_price * filled_quantity`
  - Added comprehensive logging for all balance updates
  - Included error handling with warnings about potential inconsistencies

## Order Flow (Updated)

1. **Pre-trade Validation** - Validates user has sufficient funds/holdings
2. **Order Creation** - Creates order with Pending status in Order Service
3. **Order Execution** - Executes order through Market Data Service
4. **Order Update** - Updates order with execution results
5. **Holdings Update** - Updates user's stock holdings
6. **Balance Update** (NEW):
   - **Buy**: Withdraws funds from account
   - **Sell**: Deposits funds into account
7. **Return Response** - Returns completed order to client

## Transaction Amounts

- **Buy Order**: `Debit Amount = Execution Price × Filled Quantity`
- **Sell Order**: `Credit Amount = Execution Price × Filled Quantity`

## Error Handling

- If balance update fails, the system logs an error but doesn't roll back holdings
- **Production Note**: In a production system, this should trigger compensating transactions or use a distributed transaction pattern (e.g., Saga pattern)

## API Endpoints

### New User Service Endpoint

```
POST /users/{id}/withdraw
Content-Type: application/json

{
  "amount": 1000.50
}
```

**Responses**:
- `200 OK`: Returns updated user with new balance
- `400 BAD_REQUEST`: Invalid amount or insufficient funds
- `404 NOT_FOUND`: User not found
- `500 INTERNAL_SERVER_ERROR`: Service error

## Example Scenarios

### Buy Order Execution
```
User has balance: $10,000
Order: Buy 100 shares @ $50.00
Execution: Fully filled 100 shares @ $50.00
Result:
  - Holdings: +100 shares
  - Balance: $10,000 - $5,000 = $5,000
```

### Sell Order Execution
```
User has 100 shares, balance: $5,000
Order: Sell 50 shares @ $55.00
Execution: Fully filled 50 shares @ $55.00
Result:
  - Holdings: -50 shares (50 remaining)
  - Balance: $5,000 + $2,750 = $7,750
```

### Partial Fill
```
User has balance: $10,000
Order: Buy 100 shares @ $50.00
Execution: Partially filled 60 shares @ $50.00
Result:
  - Holdings: +60 shares
  - Balance: $10,000 - $3,000 = $7,000
```

## Testing Recommendations

1. **Test buy orders** with sufficient and insufficient funds
2. **Test sell orders** with various execution prices
3. **Test partial fills** to ensure correct amount is debited/credited
4. **Test error scenarios** where balance update fails after holdings update
5. **Verify balance consistency** across multiple concurrent orders

## Future Improvements

1. **Implement distributed transactions** (Saga pattern) for atomicity
2. **Add idempotency keys** to prevent duplicate debits/credits
3. **Implement retry logic** with exponential backoff for failed balance updates
4. **Add compensating transactions** to handle partial failures
5. **Consider pre-authorization** of funds before order execution
6. **Add transaction audit log** for all balance changes

## Related Files

- `/api_gateway/src/api.rs` - Order orchestration logic
- `/api_gateway/src/clients.rs` - User service client
- `/user-service/src/api.rs` - Withdraw endpoint
- `/user-service/src/service.rs` - User service layer
- `/user-service/src/repository.rs` - Repository with withdraw method
