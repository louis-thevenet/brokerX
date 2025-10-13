# 🏦 BrokerX Complete Application Stack

## Architecture Overview

```
┌─────────────────┐    HTTP     ┌─────────────────┐    HTTP     ┌─────────────────┐
│   Web Browser   │ ◄─────────► │  API Gateway    │ ◄─────────► │ User Service    │
│  (Frontend UI)  │             │   Port: 8080    │             │   Port: 3001    │
└─────────────────┘             └─────────────────┘             └─────────────────┘
                                         │                               │
                                         │ HTTP                          │ SQL
                                         ▼                               ▼
                                ┌─────────────────┐             ┌─────────────────┐
                                │ Order Service   │             │ PostgreSQL      │
                                │   Port: 3002    │             │   Port: 5432    │
                                └─────────────────┘             │  (Users DB)     │
                                         │                      └─────────────────┘
                                         │ SQL                           
                                         ▼                               
                                ┌─────────────────┐                     
                                │ PostgreSQL      │                     
                                │   Port: 5433    │                     
                                │  (Orders DB)    │                     
                                └─────────────────┘                     
```

## Components

### 🌐 **Web Frontend** (`webapp/`)
- **Technology:** Pure HTML5 + JavaScript (ES6)
- **Features:** Complete trading UI with authentication, profile management, and order placement
- **Port:** 3000 (configurable)
- **API Integration:** Full REST API client for all BrokerX services

### 🚪 **API Gateway** (`api_gateway/`)
- **Technology:** Rust + Axum + JWT
- **Role:** Single entry point, authentication, request routing
- **Port:** 8080
- **Features:** JWT auth, pagination, input validation, OpenAPI docs

### 👤 **User Service** (`user-service/`)
- **Technology:** Rust + Axum + PostgreSQL
- **Role:** User management, authentication, profile, balance
- **Port:** 3001
- **Database:** PostgreSQL (port 5432)

### 📈 **Order Service** (`order-service/`)
- **Technology:** Rust + Axum + PostgreSQL  
- **Role:** Order placement, management, trading logic
- **Port:** 3002
- **Database:** PostgreSQL (port 5433)

### 🗄️ **Databases**
- **User Database:** PostgreSQL on port 5432
- **Order Database:** PostgreSQL on port 5433
- **Isolation:** Separate databases for microservice independence

## Quick Start

### 1. Start Infrastructure
```bash
# Start PostgreSQL databases
sudo ./start-databases.sh

# Start all microservices (user-service, order-service, api-gateway)
./start-microservices.sh

# Start web application frontend
cd webapp && ./start-webapp.sh
```

### 2. Access Application
- **Web App:** http://localhost:3000
- **API Gateway:** http://localhost:8080
- **API Docs:** http://localhost:8080/swagger-ui/

## Complete User Journey

### 1. **Registration & Authentication** (UC-01)
```
User → Web Form → API Gateway → User Service → Database
     ← JWT Token ← API Gateway ← User Created ← Database
```

### 2. **Profile & Fund Management** (UC-03)
```
User → Deposit Form → API Gateway → User Service → Update Balance
     ← New Balance  ← API Gateway ← Updated User ← Database
```

### 3. **Order Placement** (UC-05)
```
User → Order Form → API Gateway → Order Service → Create Order
     ← Order ID   ← API Gateway ← Order Created ← Database
```

### 4. **Order Management** (UC-06)
```
User → Order List → API Gateway → Order Service → Query Orders
     ← Paginated  ← API Gateway ← Order Results ← Database
```

## API Endpoints Summary

### Authentication
- `POST /auth/register` - User registration
- `POST /auth/login` - Login (returns JWT)
- `POST /auth/logout` - Logout

### User Management
- `GET /me` - Get user profile
- `PUT /me` - Update profile
- `POST /me/verify-email` - Email verification
- `POST /me/deposit` - Deposit funds

### Trading
- `POST /orders` - Place order
- `GET /orders` - List orders (paginated)
- `GET /orders/{id}` - Get order details
- `PUT /orders/{id}` - Update order
- `DELETE /orders/{id}` - Cancel order

### System
- `GET /health` - Health check
- `GET /swagger-ui/` - API documentation

## Configuration Files

### Environment Variables (`.env`)
```bash
# Database URLs
USER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5432/brokerx_users
ORDER_DATABASE_URL=postgresql://brokerx_user:password123@localhost:5433/brokerx_orders

# Service Ports
USER_SERVICE_PORT=3001
ORDER_SERVICE_PORT=3002
API_GATEWAY_PORT=8080

# Service URLs (for API Gateway)
USER_SERVICE_URL=http://localhost:3001
ORDER_SERVICE_URL=http://localhost:3002
API_GATEWAY_ADDRESS=0.0.0.0:8080

# JWT Configuration
JWT_SECRET=super-secret-jwt
JWT_EXPIRY_HOURS=24

# Email Configuration (for user service)
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=your-email@gmail.com
SMTP_SERVER=smtp.gmail.com
SMTP_PORT=587
```

## Development Workflow

### Adding New Features

1. **API Changes:**
   - Update models in `api_gateway/src/models.rs`
   - Add handlers in `api_gateway/src/api.rs`
   - Update client in `api_gateway/src/clients.rs`

2. **Service Changes:**
   - Implement business logic in respective service
   - Update database schema if needed
   - Add API endpoints

3. **Frontend Changes:**
   - Add UI elements in `webapp/index.html`
   - Implement logic in `webapp/app.js`
   - Handle new API endpoints

### Testing

1. **Manual Testing:**
   - Use web app at http://localhost:3000
   - Use Swagger UI at http://localhost:8080/swagger-ui/

2. **API Testing:**
   ```bash
   # Health check
   curl http://localhost:8080/health
   
   # Register user
   curl -X POST http://localhost:8080/auth/register \
     -H "Content-Type: application/json" \
     -d '{"email":"test@example.com","password":"password123","firstname":"Test","surname":"User","initial_balance":1000}'
   ```

## Security Features

- **JWT Authentication:** Stateless tokens for API access
- **Password Hashing:** Secure password storage in user service
- **User Isolation:** Users can only access their own data
- **Input Validation:** Request validation at API Gateway
- **Database Separation:** Isolated databases per service
- **CORS Configuration:** Configurable cross-origin policies

## Production Considerations

### Security
- Use HTTPS in production
- Change JWT secret to a secure random value
- Configure restrictive CORS policies
- Set up proper database credentials
- Enable rate limiting

### Scalability  
- Load balance API Gateway instances
- Scale microservices independently
- Use database connection pooling
- Add caching layer (Redis)
- Implement circuit breakers

### Monitoring
- Add logging middleware
- Implement health checks
- Set up metrics collection
- Add distributed tracing
- Monitor database performance

## Implemented Use Cases

✅ **UC-01:** User Registration & Verification  
✅ **UC-03:** Fund Deposit (Virtual)  
✅ **UC-05:** Order Placement with Pre-trade Checks  
✅ **UC-06:** Order Modification & Cancellation  
🔄 **UC-07:** Order Matching & Execution (Future)  
🔄 **UC-08:** Execution Confirmation & Notifications (Future)  

The system provides a solid foundation for a complete trading platform with room for additional features like market data feeds, order matching engines, and notification systems.
