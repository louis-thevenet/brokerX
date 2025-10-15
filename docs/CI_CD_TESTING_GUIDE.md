# CI/CD and Testing Guide

## Overview

BrokerX uses GitHub Actions for continuous integration with a microservices architecture requiring multiple PostgreSQL databases.

## Architecture

The project consists of the following microservices, each with its own database:

1. **User Service** - Port 3001, Database: `brokerx_users` (Port 5432)
2. **Order Service** - Port 3002, Database: `brokerx_orders` (Port 5433)
3. **Market Data Service** - Port 3003, Database: `brokerx_market_data` (Port 5434)
4. **API Gateway** - Port 8080, Routes requests to services

## Local Development Setup

### Using Docker Compose (Recommended)

```bash
# Start all databases
just db

# Or manually:
docker compose up -d user-db order-db market-data-db

# Wait for databases to be ready
docker compose ps
```

The databases will be initialized with schemas from:
- `docker/init-user-db.sql`
- `docker/init-order-db.sql`
- `docker/init-market-data-db.sql`

### Database Ports

| Database | Host Port | Container Port | Database Name |
|----------|-----------|----------------|---------------|
| User DB | 5432 | 5432 | brokerx_users |
| Order DB | 5433 | 5432 | brokerx_orders |
| Market Data DB | 5434 | 5432 | brokerx_market_data |

### Environment Variables for Testing

Each service supports service-specific database URLs with fallback to `DATABASE_URL`:

```bash
# Service-specific URLs (preferred)
export USER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5432/brokerx_users
export ORDER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5433/brokerx_orders
export MARKET_DATA_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5434/brokerx_market_data

# Fallback (for backward compatibility)
export DATABASE_URL=postgresql://brokerx_user:password123@localhost:5432/brokerx_users
```

## Running Tests Locally

### Run All Tests

```bash
# Start databases first
just db

# Run all tests
cargo test --all-features --workspace
```

### Run Tests for Specific Service

```bash
# User service tests
cargo test -p user-service

# Order service tests
cargo test -p order-service

# Market data service tests
cargo test -p market-data-service

# API gateway tests
cargo test -p api_gateway
```

### Run Tests with Logging

```bash
# Show test output
cargo test -- --nocapture

# Show debug logs
RUST_LOG=debug cargo test
```

### Test Isolation

Tests use temporary tables to avoid conflicts:
- Each test creates a unique table name using UUID
- Tables are automatically prefixed (e.g., `users_test_abc123def456`)
- Tests can run in parallel without interference

## CI/CD Pipeline

### GitHub Actions Workflow

Location: `.github/workflows/ci.yml`

The CI pipeline:

1. **Sets up 3 PostgreSQL services** as GitHub Actions services
   - Each database runs in its own container
   - Health checks ensure databases are ready before tests
   - Different ports avoid conflicts (5432, 5433, 5434)

2. **Initializes databases with schemas**
   - Runs SQL init scripts for each database
   - Creates tables and indexes
   - Inserts sample data for market-data-service

3. **Runs tests** with proper environment variables
   - All service-specific DATABASE_URLs are set
   - Fallback DATABASE_URL for backward compatibility
   - Tests run in parallel using Cargo's test runner

4. **Additional checks**:
   - `rustfmt` - Code formatting
   - `docs` - Documentation generation

### Environment Variables in CI

```yaml
env:
  CI: true
  USER_DATABASE_URL: postgresql://brokerx_user:password123@localhost:5432/brokerx_users
  ORDER_DATABASE_URL: postgresql://brokerx_user:password123@localhost:5433/brokerx_orders
  MARKET_DATA_DATABASE_URL: postgresql://brokerx_user:password123@localhost:5434/brokerx_market_data
  DATABASE_URL: postgresql://brokerx_user:password123@localhost:5432/brokerx_users
```

## Database Initialization Scripts

### User Database (`docker/init-user-db.sql`)
- Grants permissions
- Sets up user service schema
- Minimal initialization (tables created by service on startup)

### Order Database (`docker/init-order-db.sql`)
- Creates `orders` table with proper schema
- Creates indexes for performance:
  - `idx_orders_client_id` - Find orders by user
  - `idx_orders_status` - Filter by status
  - `idx_orders_client_status` - Composite index
  - `idx_orders_client_date` - Date sorting
  - `idx_orders_symbol` - Symbol lookup
- Sets up update triggers

### Market Data Database (`docker/init-market-data-db.sql`)
- Creates `market_prices` table
- Creates indexes for efficient querying
- Inserts sample data for testing (AAPL, GOOGL, MSFT, etc.)

## Running Services

### Start All Services

```bash
# Option 1: Using Justfile (recommended)
just services

# Option 2: Manually in separate terminals
cargo run --release -p user-service
cargo run --release -p order-service
cargo run --release -p market-data-service
cargo run --release -p api_gateway
```

### Service Endpoints

- User Service: http://localhost:3001
- Order Service: http://localhost:3002
- Market Data Service: http://localhost:3003
- API Gateway: http://localhost:8080
- Swagger UI: http://localhost:8080/swagger-ui/

### Health Checks

Each service provides a health endpoint:

```bash
# Check service health
curl http://localhost:3001/health  # User service
curl http://localhost:3002/health  # Order service
curl http://localhost:3003/health  # Market data service
curl http://localhost:8080/health  # API gateway
```

## Troubleshooting

### Database Connection Issues

```bash
# Check if databases are running
docker compose ps

# Check database logs
docker compose logs user-db
docker compose logs order-db
docker compose logs market-data-db

# Restart databases
docker compose restart user-db order-db market-data-db
```

### Test Failures

```bash
# Run tests with verbose output
cargo test -- --nocapture

# Run a specific test
cargo test test_name -- --exact --nocapture

# Check database connectivity
psql -h localhost -p 5432 -U brokerx_user -d brokerx_users
psql -h localhost -p 5433 -U brokerx_user -d brokerx_orders
psql -h localhost -p 5434 -U brokerx_user -d brokerx_market_data
```

### Port Conflicts

If you have existing PostgreSQL instances:

```bash
# Check what's using the ports
lsof -i :5432
lsof -i :5433
lsof -i :5434

# Stop conflicting services or change docker-compose.yml ports
```

## Best Practices

### Writing Tests

1. **Use temporary tables**: Each test should create unique table names
2. **Clean up resources**: Tests should be self-contained
3. **Use service-specific env vars**: Prefer `USER_DATABASE_URL` over `DATABASE_URL`
4. **Mock external dependencies**: API gateway tests should mock service calls
5. **Test isolation**: Don't rely on test execution order

### Database Migrations

When adding new tables or columns:

1. Update the SQL init script in `docker/`
2. Update the service's database schema handling
3. Test with a fresh database: `docker compose down -v && docker compose up -d`

### CI Performance

- Rust compilation cache speeds up builds (~5-10x faster)
- Database health checks prevent flaky tests
- Parallel test execution is enabled by default

## Additional Resources

- [Docker Compose Configuration](../docker-compose.yml)
- [Justfile Commands](../Justfile)
- [Environment Setup Guide](../ENV_SETUP.md)
- [Microservices Architecture](../MICROSERVICES.md)
