# Market Data Service

A microservice for simulating and managing market price data in the BrokerX trading platform.

## Overview

The Market Data Service provides real-time and historical pricing information for financial assets. It follows REST API best practices and integrates seamlessly with the BrokerX microservices architecture.

## Features

- **Real-time Price Data**: Get current prices for individual or multiple symbols
- **Historical Data**: Query historical price data with flexible date range filtering
- **Price Simulation**: Simulate realistic market price movements
- **Symbol Management**: List all available trading symbols
- **REST API**: Full RESTful API with OpenAPI/Swagger documentation
- **Database Persistence**: Store historical price data in PostgreSQL

## API Endpoints

### Price Operations

- `GET /api/v1/prices/{symbol}` - Get current price for a specific symbol
- `GET /api/v1/prices?symbols=AAPL,GOOGL` - Get current prices for multiple symbols
- `GET /api/v1/prices` - Get all current prices
- `GET /api/v1/prices/{symbol}/history` - Get historical prices with optional filtering

### Symbol Operations

- `GET /api/v1/symbols` - List all available symbols

### Administrative Operations

- `POST /api/v1/initialize` - Initialize market data with default symbols
- `POST /api/v1/simulate` - Simulate price updates for all symbols

### Health Check

- `GET /health` - Service health status

## Configuration

The service uses environment variables for configuration:

```bash
# Service Configuration
MARKET_DATA_SERVICE_HOST=0.0.0.0
MARKET_DATA_SERVICE_PORT=3003

# Database Configuration
MARKET_DATA_DB_HOST=localhost
MARKET_DATA_DB_PORT=5434
MARKET_DATA_DB_USER=brokerx_user
MARKET_DATA_DB_PASSWORD=password123
MARKET_DATA_DB_NAME=brokerx_market_data
```

## Running the Service

### With Docker Compose

```bash
docker-compose up market-data-db market-data-service
```

### Standalone

```bash
# Set environment variables
export MARKET_DATA_SERVICE_PORT=3003
export MARKET_DATA_DB_HOST=localhost
export MARKET_DATA_DB_PORT=5434

# Run the service
cargo run --bin market-data-service
```

## Database Schema

The service uses a PostgreSQL database with the following schema:

```sql
CREATE TABLE market_prices (
    id UUID PRIMARY KEY,
    symbol VARCHAR(10) NOT NULL,
    price DECIMAL(12, 2) NOT NULL,
    open DECIMAL(12, 2) NOT NULL,
    high DECIMAL(12, 2) NOT NULL,
    low DECIMAL(12, 2) NOT NULL,
    volume BIGINT NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL
);
```

## Initial Setup

1. Start the database:
   ```bash
   docker-compose up -d market-data-db
   ```

2. Initialize market data with default symbols:
   ```bash
   curl -X POST http://localhost:3003/api/v1/initialize
   ```

3. Start price simulation (optional):
   ```bash
   curl -X POST http://localhost:3003/api/v1/simulate
   ```

## API Documentation

Interactive API documentation is available via Swagger UI:

```
http://localhost:3003/swagger-ui
```

## Architecture

The service follows a layered architecture:

```
API Layer (api.rs)
    ↓
Service Layer (service.rs)
    ↓
Repository Layer (repository.rs)
    ↓
Database (PostgreSQL)
```

## Example Usage

### Get Current Price

```bash
curl http://localhost:3003/api/v1/prices/AAPL
```

Response:
```json
{
  "symbol": "AAPL",
  "price": 175.50,
  "open": 174.20,
  "high": 176.80,
  "low": 173.90,
  "volume": 1500000,
  "timestamp": "2024-10-13T12:00:00Z"
}
```

### Get Multiple Prices

```bash
curl "http://localhost:3003/api/v1/prices?symbols=AAPL,GOOGL,MSFT"
```

### Get Historical Data

```bash
curl "http://localhost:3003/api/v1/prices/AAPL/history?limit=10"
```

## Integration with Other Services

The Market Data Service can be integrated with:

- **Order Service**: Fetch current prices for order validation
- **API Gateway**: Proxy requests to market data endpoints
- **Web Application**: Display real-time prices to users

## Testing

Run tests with:

```bash
cargo test --package market-data-service
```

## License

MIT
