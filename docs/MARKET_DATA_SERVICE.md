# Market Data Service Integration

## Overview

The Market Data Service is a new microservice in the BrokerX architecture that provides simulated market price data for financial assets. This document describes the service and its integration with the rest of the system.

## Architecture

### Service Structure

```
market-data-service/
├── src/
│   ├── main.rs          # Service entry point
│   ├── lib.rs           # Library exports
│   ├── api.rs           # REST API handlers
│   ├── service.rs       # Business logic
│   ├── repository.rs    # Database access
│   ├── models.rs        # Domain models
│   └── config.rs        # Configuration
├── migrations/          # Database migrations
├── Cargo.toml          # Dependencies
└── README.md           # Service documentation
```

### Database Schema

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

## REST API Endpoints

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/api/v1/prices/{symbol}` | Get current price for a symbol |
| GET | `/api/v1/prices?symbols=A,B` | Get current prices for multiple symbols |
| GET | `/api/v1/prices` | Get all current prices |
| GET | `/api/v1/prices/{symbol}/history` | Get historical prices |
| GET | `/api/v1/symbols` | List available symbols |

### Administrative Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/initialize` | Initialize with default symbols |
| POST | `/api/v1/simulate` | Simulate price updates |

## API Examples

### Get Current Price

**Request:**
```bash
GET /api/v1/prices/AAPL
```

**Response:**
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

**Request:**
```bash
GET /api/v1/prices?symbols=AAPL,GOOGL,MSFT
```

**Response:**
```json
{
  "prices": [
    {
      "symbol": "AAPL",
      "price": 175.50,
      "open": 174.20,
      "high": 176.80,
      "low": 173.90,
      "volume": 1500000,
      "timestamp": "2024-10-13T12:00:00Z"
    },
    ...
  ],
  "count": 3
}
```

### Get Historical Prices

**Request:**
```bash
GET /api/v1/prices/AAPL/history?limit=10
```

**Response:**
```json
{
  "symbol": "AAPL",
  "prices": [...],
  "count": 10
}
```

## Integration Points

### 1. Order Service Integration

The Order Service should call the Market Data Service to:
- Validate market orders against current prices
- Retrieve prices for order execution
- Display current prices to users

**Example Integration:**
```rust
// In order-service
async fn validate_order_price(symbol: &str) -> Result<f64> {
    let response = reqwest::get(
        format!("http://localhost:3003/api/v1/prices/{}", symbol)
    ).await?;
    
    let price_data: MarketPriceResponse = response.json().await?;
    Ok(price_data.price)
}
```

### 2. API Gateway Integration

The API Gateway should proxy market data requests:

```rust
// In api_gateway/src/api.rs
async fn get_market_prices() -> Result<Json<MarketPricesResponse>> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:3003/api/v1/prices")
        .send()
        .await?;
    
    let prices = response.json().await?;
    Ok(Json(prices))
}
```

### 3. Web Application Integration

The web application can fetch prices for display:

```javascript
// In webapp
async function fetchMarketPrices() {
    const response = await fetch('http://localhost:8080/api/prices');
    const data = await response.json();
    return data.prices;
}
```

## Configuration

### Environment Variables

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

### Docker Compose

The service is configured in `docker-compose.yml`:

```yaml
market-data-db:
  image: postgres:15
  environment:
    POSTGRES_DB: brokerx_market_data
    POSTGRES_USER: brokerx_user
    POSTGRES_PASSWORD: password123
  ports:
    - "5434:5432"
```

## Deployment

### 1. Start the Database

```bash
./start-databases.sh
```

This will start:
- User DB (port 5432)
- Order DB (port 5433)
- Market Data DB (port 5434)

### 2. Initialize Market Data

```bash
curl -X POST http://localhost:3003/api/v1/initialize
```

This creates initial price data for common symbols (AAPL, GOOGL, MSFT, etc.).

### 3. Start Price Simulation (Optional)

```bash
curl -X POST http://localhost:3003/api/v1/simulate
```

This generates new price data with realistic variations.

### 4. Start All Services

```bash
./start-microservices.sh
```

This starts:
- User Service (port 3001)
- Order Service (port 3002)
- Market Data Service (port 3003)
- API Gateway (port 8080)

## Price Simulation Algorithm

The service uses a simple but realistic price simulation:

1. **Price Change**: Random variation between -2% and +2%
2. **Open Price**: Based on previous price with small variation
3. **High/Low**: Calculated based on open and current price
4. **Volume**: Random variation between 80% and 120% of previous volume

This provides realistic market movement for testing and demonstration.

## Default Symbols

The service initializes with the following symbols:

| Symbol | Name | Initial Price |
|--------|------|---------------|
| AAPL | Apple Inc. | $175.50 |
| GOOGL | Alphabet Inc. | $140.25 |
| MSFT | Microsoft Corp. | $380.75 |
| AMZN | Amazon.com Inc. | $145.30 |
| TSLA | Tesla Inc. | $245.60 |
| META | Meta Platforms | $325.40 |
| NVDA | NVIDIA Corp. | $485.90 |
| NFLX | Netflix Inc. | $455.20 |

## API Documentation

Interactive API documentation is available at:

```
http://localhost:3003/swagger-ui
```

## Future Enhancements

Possible improvements for production use:

1. **Real-time Updates**: WebSocket support for live price streaming
2. **External Data**: Integration with real market data providers
3. **Advanced Simulation**: More sophisticated price models
4. **Caching**: Redis cache for frequently accessed prices
5. **Rate Limiting**: Protect against excessive requests
6. **Authentication**: Secure administrative endpoints
7. **Metrics**: Prometheus metrics for monitoring
8. **Historical Aggregation**: OHLC (Open, High, Low, Close) data aggregation

## Testing

### Manual Testing

```bash
# Get current prices
curl http://localhost:3003/api/v1/prices

# Get specific symbol
curl http://localhost:3003/api/v1/prices/AAPL

# Get historical data
curl "http://localhost:3003/api/v1/prices/AAPL/history?limit=5"

# List symbols
curl http://localhost:3003/api/v1/symbols

# Initialize data
curl -X POST http://localhost:3003/api/v1/initialize

# Simulate updates
curl -X POST http://localhost:3003/api/v1/simulate
```

### Automated Testing

```bash
cargo test --package market-data-service
```

## Best Practices

1. **Always check symbol availability** before requesting prices
2. **Use batch requests** when fetching multiple prices
3. **Cache prices** on the client side when appropriate
4. **Handle 404 errors** for unknown symbols
5. **Use historical data** for backtesting and analysis

## Troubleshooting

### Service won't start
- Check if port 3003 is available
- Verify database connection
- Check environment variables

### No data returned
- Run initialization: `POST /api/v1/initialize`
- Check database is running
- Verify migrations ran successfully

### Stale prices
- Run simulation: `POST /api/v1/simulate`
- Check timestamp in responses

## Support

For questions or issues:
- Check the service logs
- Review the API documentation at `/swagger-ui`
- Consult the main BrokerX documentation
