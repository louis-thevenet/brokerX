# Environment Configuration Guide

## Overview

BrokerX uses environment variables to configure services. There are two deployment scenarios:

### 1. **Local Development (All Services Together)**
Use the root `.env` file which contains configuration for all services.

```bash
# Copy the example file
cp .env.example .env

# Edit with your actual values
vim .env

# Start all services
just services
```

### 2. **Independent Deployment (Each Service on Different Servers)**
Each service has its own `.env.example` file in its directory.

```bash
# For user-service
cd user-service
cp .env.example .env
# Edit with your actual values
vim .env
cargo run --release

# For order-service
cd order-service
cp .env.example .env
# Edit with your actual values and update service URLs
vim .env
cargo run --release

# Repeat for market-data-service and api_gateway
```

## Service-Specific Configuration

### User Service
**File**: `user-service/.env.example`
- Database: `USER_DATABASE_URL`
- SMTP settings for email OTP
- Service port: `USER_SERVICE_PORT`

### Order Service
**File**: `order-service/.env.example`
- Database: `ORDER_DATABASE_URL`
- Service port: `ORDER_SERVICE_PORT`
- **Note**: Pure data service - no external dependencies, all validation done by API Gateway

### Market Data Service
**File**: `market-data-service/.env.example`
- Database: `MARKET_DATA_DATABASE_URL`
- Service port: `MARKET_DATA_SERVICE_PORT`

### API Gateway
**File**: `api_gateway/.env.example`
- All service URLs
- JWT configuration
- Gateway port: `API_GATEWAY_PORT`

## Security Notes

1. **Never commit `.env` files** - they contain secrets
2. **Change JWT_SECRET** in production
3. **Use app passwords** for SMTP (not your main password)
4. **Update service URLs** when deploying to different servers
