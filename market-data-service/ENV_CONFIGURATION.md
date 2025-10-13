# Market Data Service - Environment Configuration

## Overview

The Market Data Service follows the same environment variable configuration pattern as the other BrokerX microservices (user-service and order-service), ensuring consistency across the entire platform.

## Configuration Pattern

### Environment Variables

The service uses the following environment variables:

```bash
# Service Configuration
MARKET_DATA_SERVICE_HOST=0.0.0.0
MARKET_DATA_SERVICE_PORT=3003

# Database Configuration
MARKET_DATA_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5434/brokerx_market_data

# Fallback (for backward compatibility)
DATABASE_URL=postgresql://brokerx_user:password123@localhost:5432/brokerx_users
```

### Configuration Loading

The service loads configuration in the following order:

1. **Primary**: `MARKET_DATA_DATABASE_URL` - Dedicated market data database URL
2. **Fallback**: `DATABASE_URL` - Shared database URL (for development/testing)
3. **Error**: If neither is set, the service will fail to start with a clear error message

This pattern matches exactly how `user-service` uses `USER_DATABASE_URL` and `order-service` uses `ORDER_DATABASE_URL`.

## Configuration Files

### Root Configuration: `.env.microservices`

All microservices share a centralized configuration file at the workspace root:

```bash
# Database URLs - Docker containers
USER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5432/brokerx_users
ORDER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5433/brokerx_orders
MARKET_DATA_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5434/brokerx_market_data

# Service ports
USER_SERVICE_PORT=3001
ORDER_SERVICE_PORT=3002
MARKET_DATA_SERVICE_PORT=3003

# Service URLs for inter-service communication
USER_SERVICE_URL=http://localhost:3001
ORDER_SERVICE_URL=http://localhost:3002
MARKET_DATA_SERVICE_URL=http://localhost:3003
```

### Service-Specific: `.env.example`

The market-data-service directory contains a `.env.example` file for reference, but it redirects to the root configuration:

```bash
# Note: This file is for reference only.
# The actual configuration should be in the root .env file of the workspace.
```

## Database Setup

### Docker Compose

The service uses a dedicated PostgreSQL database defined in `docker-compose.yml`:

```yaml
market-data-db:
  image: postgres:15
  container_name: brokerx-market-data-db
  environment:
    POSTGRES_DB: brokerx_market_data
    POSTGRES_USER: brokerx_user
    POSTGRES_PASSWORD: password123
  ports:
    - "5434:5432"  # Unique port for market data DB
```

### Database Initialization

The database is initialized with the `docker/init-market-data-db.sql` script, which creates the necessary tables and indexes.

## Code Implementation

### Config Module (`src/config.rs`)

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let port = std::env::var("MARKET_DATA_SERVICE_PORT")
            .unwrap_or_else(|_| "3003".to_string())
            .parse()
            .expect("MARKET_DATA_SERVICE_PORT must be a valid port number");

        let host = std::env::var("MARKET_DATA_SERVICE_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        Self { port, host }
    }
}
```

### Main Module (`src/main.rs`)

```rust
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Load configuration
    let config = config::Config::from_env();

    // Use MARKET_DATA_DATABASE_URL or fall back to DATABASE_URL
    let market_data_db_url = std::env::var("MARKET_DATA_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("Either MARKET_DATA_DATABASE_URL or DATABASE_URL must be set");
    let db_config = DatabaseConfig::from_url(market_data_db_url);

    // ... rest of initialization
}
```

## Consistency with Other Services

| Aspect | User Service | Order Service | Market Data Service |
|--------|-------------|---------------|-------------------|
| Port Var | `USER_SERVICE_PORT` | `ORDER_SERVICE_PORT` | `MARKET_DATA_SERVICE_PORT` |
| Host Var | `USER_SERVICE_HOST` | `ORDER_SERVICE_HOST` | `MARKET_DATA_SERVICE_HOST` |
| DB Var | `USER_DATABASE_URL` | `ORDER_DATABASE_URL` | `MARKET_DATA_DATABASE_URL` |
| Fallback | `DATABASE_URL` | `DATABASE_URL` | `DATABASE_URL` |
| Default Port | 3001 | 3002 | 3003 |
| DB Port | 5432 | 5433 | 5434 |

## Best Practices

1. **Centralized Configuration**: All environment variables are defined in the root `.env.microservices` file
2. **Consistent Naming**: Service-specific variables follow the pattern `{SERVICE}_SERVICE_{PROPERTY}`
3. **Graceful Fallbacks**: Services fall back to `DATABASE_URL` for development/testing scenarios
4. **Clear Errors**: Services fail fast with descriptive error messages if required variables are missing
5. **Documentation**: Each service includes reference `.env.example` files that point to the root configuration

## Usage

### Starting the Service

```bash
# Ensure the root .env.microservices is configured
cd /path/to/brokerx

# Start all databases
./start-databases.sh

# Start all microservices (includes market-data-service)
./start-microservices.sh
```

### Setup and Testing

```bash
# Initialize market data with sample symbols and prices
cd market-data-service
./setup-market-data.sh
# or
./setup-market-data.fish
```

## Migration from Previous Configuration

If you were using the old configuration format with individual DB connection parameters (`MARKET_DATA_DB_HOST`, `MARKET_DATA_DB_PORT`, etc.), update to use the new URL format:

**Old:**
```bash
MARKET_DATA_DB_HOST=localhost
MARKET_DATA_DB_PORT=5434
MARKET_DATA_DB_USER=brokerx_user
MARKET_DATA_DB_PASSWORD=password123
MARKET_DATA_DB_NAME=brokerx_market_data
```

**New:**
```bash
MARKET_DATA_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5434/brokerx_market_data
```

This change brings the market data service in line with the other microservices and simplifies configuration management.
