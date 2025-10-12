# BrokerX Microservices Architecture

This document explains the microservices architecture implementation for BrokerX.

## Architecture Overview

BrokerX has been refactored from a monolithic application into a microservices architecture:

```
┌─────────────────┐    ┌─────────────────┐
│   User Service  │    │  Order Service  │
│    Port: 3001   │    │    Port: 3002   │ 
│                 │    │                 │
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │   User DB   │ │    │ │  Order DB   │ │
│ │             │ │    │ │             │ │
│ └─────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └─────────────────┘
         │                       │
         └───────────┬───────────┘
                     │
              HTTP API Calls
```

## Services

### User Service (Port 3001)
**Responsibility**: User management, authentication, account operations

**API Endpoints**:
- `GET /health` - Health check
- `POST /users` - Create user
- `GET /users/{id}` - Get user by ID
- `PUT /users/{id}` - Update user
- `GET /users/by-email/{email}` - Get user by email
- `POST /users/{id}/verify` - Verify user email
- `POST /auth/login` - Authenticate user

**Database**: Dedicated user database (`brokerx_users`)

### Order Service (Port 3002)
**Responsibility**: Order management, order processing, trade execution

**API Endpoints**:
- `GET /health` - Health check
- `POST /orders` - Create order
- `GET /orders/{id}` - Get order by ID
- `GET /users/{user_id}/orders` - Get orders for user

**Database**: Dedicated order database (`brokerx_orders`)

**Dependencies**: Calls User Service for user validation

## Inter-Service Communication

The Order Service communicates with the User Service via HTTP REST API calls:

```rust
// Order service validates users via HTTP
let user_exists = user_client.user_exists(&user_id).await?;
```

This replaces the previous direct database access pattern and provides true service isolation.

## Getting Started

### 1. Database Setup

Create separate databases:
```sql
CREATE DATABASE brokerx_users;
CREATE DATABASE brokerx_orders;
```

### 2. Environment Configuration

Copy the microservices environment template:
```bash
cp .env.microservices .env
# Edit .env with your database credentials
```

### 3. Start Services

Use the provided startup script:
```bash
./start-microservices.sh
```

Or start manually:
```bash
# Terminal 1 - User Service
cargo run -p user-service

# Terminal 2 - Order Service  
cargo run -p order-service
```

### 4. Test the Services

Visit the Swagger UI documentation:
- User Service: http://localhost:3001/swagger-ui
- Order Service: http://localhost:3002/swagger-ui

## Development

### Building Services
```bash
# Build all services
cargo build -p user-service -p order-service

# Build for production
cargo build --release -p user-service -p order-service
```

### Testing Services
```bash
# Test individual service
cargo test -p user-service
cargo test -p order-service

# Test domain logic
cargo test -p domain
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `USER_SERVICE_PORT` | User service port | 3001 |
| `ORDER_SERVICE_PORT` | Order service port | 3002 |
| `USER_DATABASE_URL` | User service database | Uses `DATABASE_URL` |
| `ORDER_DATABASE_URL` | Order service database | Uses `DATABASE_URL` |
| `USER_SERVICE_URL` | User service URL for inter-service calls | http://localhost:3001 |

## Benefits Achieved

1. **Service Isolation**: Each service has its own database and codebase
2. **Independent Deployment**: Services can be deployed separately
3. **Technology Flexibility**: Services can use different tech stacks
4. **Scalability**: Services can be scaled independently
5. **Fault Tolerance**: Failure in one service doesn't bring down others
6. **Team Autonomy**: Different teams can own different services

## Migration Path

- ✅ **Phase 1**: Domain separation within monolith
- ✅ **Phase 2**: Separate databases per domain  
- ✅ **Phase 3**: Extract services to standalone binaries
- 🔄 **Phase 4**: Add API Gateway, service mesh
- 🔄 **Phase 5**: Add monitoring, distributed tracing
- 🔄 **Phase 6**: Container orchestration (Docker + Kubernetes)

## Known Limitations

1. **No Circuit Breakers**: Services don't handle downstream failures gracefully yet
2. **No Service Discovery**: Hard-coded service URLs
3. **Limited Error Handling**: Basic HTTP error responses
4. **No Authentication**: Services trust each other completely
5. **Single Point of Failure**: No redundancy or load balancing

These will be addressed in future iterations as the microservices mature.
