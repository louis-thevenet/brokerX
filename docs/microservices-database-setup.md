# Microservices Database Configuration

This document explains how to configure separate databases for the user and order microservices.

## Environment Variables

### Single Database (Legacy)
```bash
DATABASE_URL=postgresql://username:password@localhost/brokerx
```

### Separate Databases (Microservices)
```bash
# User service database
USER_DATABASE_URL=postgresql://username:password@localhost/brokerx_users

# Order service database  
ORDER_DATABASE_URL=postgresql://username:password@localhost/brokerx_orders

# Fallback - if microservice URLs aren't set, both will use this
DATABASE_URL=postgresql://username:password@localhost/brokerx
```

## Database Setup

### Option 1: Create separate databases
```sql
-- Connect to PostgreSQL as admin user
CREATE DATABASE brokerx_users;
CREATE DATABASE brokerx_orders;

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE brokerx_users TO your_app_user;
GRANT ALL PRIVILEGES ON DATABASE brokerx_orders TO your_app_user;
```

### Option 2: Use schemas in single database (simpler setup)
```sql
-- Connect to your main database
CREATE SCHEMA user_service;
CREATE SCHEMA order_service;

-- Grant permissions
GRANT ALL ON SCHEMA user_service TO your_app_user;
GRANT ALL ON SCHEMA order_service TO your_app_user;
```

Then update your URLs to:
```bash
USER_DATABASE_URL=postgresql://username:password@localhost/brokerx?options=-csearch_path%3Duser_service
ORDER_DATABASE_URL=postgresql://username:password@localhost/brokerx?options=-csearch_path%3Dorder_service
```

## Usage in Code

### Using BrokerX with separate databases
```rust
// New method that uses separate databases
let broker = BrokerX::new_with_separate_databases().await;

// Or specify thread count
let broker = BrokerX::with_separate_databases_and_thread_count(8).await;
```

### Legacy single database (still supported)
```rust
let broker = BrokerX::new().await;
```

## Testing

For tests, the system automatically creates separate test databases:
- `brokerx_user_test` for user service tests
- `brokerx_order_test` for order service tests

This ensures test isolation between the microservices.

## Benefits

1. **Data Isolation**: User and order data are completely separated
2. **Independent Scaling**: Each database can be scaled independently
3. **Service Autonomy**: Each microservice owns its data
4. **Security**: Reduced blast radius if one database is compromised
5. **Performance**: Specialized database configurations for each domain

## Migration Path

1. **Phase 1** (Current): Single application, separate databases
2. **Phase 2** (Future): Extract to separate HTTP/gRPC services
3. **Phase 3** (Future): Add service mesh, monitoring, etc.
