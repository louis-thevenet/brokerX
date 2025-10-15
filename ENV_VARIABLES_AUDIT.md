# Environment Variables - Actual Usage Summary

## What Each Service Actually Uses

### User Service
**Files checked**: `user-service/src/config.rs`, `user-service/src/main.rs`, `mfa_adapter/src/lib.rs`

**Required**:
- `USER_DATABASE_URL` (or fallback to `DATABASE_URL`)
- `SMTP_USERNAME` - For email OTP
- `SMTP_PASSWORD` - For email OTP
- `SMTP_FROM_EMAIL` - For email OTP

**Optional** (with defaults):
- `USER_SERVICE_PORT` (default: 3001)
- `USER_SERVICE_HOST` (default: 0.0.0.0)
- `SMTP_SERVER` (default: smtp.gmail.com)
- `SMTP_PORT` (default: 587)
- `SMTP_FROM_NAME` (default: BrokerX Security)

### Order Service
**Files checked**: `order-service/src/config.rs`, `order-service/src/main.rs`, `order-service/src/service.rs`

**Required**:
- `ORDER_DATABASE_URL` (or fallback to `DATABASE_URL`)

**Optional** (with defaults):
- `ORDER_SERVICE_PORT` (default: 3002)
- `ORDER_SERVICE_HOST` (default: 0.0.0.0)

**Architecture**: Order Service is now a pure data service with NO external dependencies:
- No validation (API Gateway does pre-trade checks)
- No calls to other services
- Simply manages order CRUD operations in its database
- Completely standalone and stateless

### Market Data Service
**Files checked**: `market-data-service/src/config.rs`, `market-data-service/src/main.rs`

**Required**:
- `MARKET_DATA_DATABASE_URL` (or fallback to `DATABASE_URL`)

**Optional** (with defaults):
- `MARKET_DATA_SERVICE_PORT` (default: 3003)
- `MARKET_DATA_SERVICE_HOST` (default: 0.0.0.0)

**Note**: Market Data Service is completely standalone and doesn't call any other services.

### API Gateway
**Files checked**: `api_gateway/src/config.rs`

**Required**:
- `JWT_SECRET` (default provided but should be changed in production)

**Optional** (with defaults):
- `API_GATEWAY_ADDRESS` (default: 0.0.0.0:8080)
- `USER_SERVICE_URL` (default: http://localhost:3001)
- `ORDER_SERVICE_URL` (default: http://localhost:3002)
- `MARKET_DATA_SERVICE_URL` (default: http://localhost:3003)
- `JWT_EXPIRY_HOURS` (default: 24)

## Key Findings

1. **Order Service is now a pure data service** - No validation, no external calls
2. **API Gateway handles all business logic** - Pre-trade validation, orchestration, holdings updates
3. All services have fallback to `DATABASE_URL` if their specific URL is not set
4. API Gateway is the orchestrator that knows all service URLs
5. SMTP is only needed by User Service (via mfa_adapter)
6. **Clear separation of concerns**:
   - API Gateway = Business logic + orchestration
   - Order Service = Data persistence only
   - User Service = User data + authentication
   - Market Data Service = Market information

## Deployment Scenarios

### Local Development
Use root `.env` with all variables - everything defaults to localhost.

### Distributed Deployment
Each service only needs:
- Its own database URL
- Its own port/host
- **No service-to-service URLs needed** (only API Gateway knows backend service URLs)
- User/Order/Market Data → completely standalone data services
- API Gateway → needs all service URLs (it's the orchestrator)
