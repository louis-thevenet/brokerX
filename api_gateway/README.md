# API Gateway

The API Gateway serves as the single entry point for all client requests in the BrokerX microservices architecture. It handles authentication, request routing, and provides a unified REST API interface.

## Features

- **Authentication & Authorization**: JWT-based authentication with Bearer token support
- **Request Routing**: Routes requests to appropriate microservices (user-service, order-service)
- **Pagination**: Built-in pagination support for list endpoints handled at the microservice level
- **API Documentation**: Auto-generated OpenAPI/Swagger documentation
- **CORS Support**: Configurable CORS for web clients
- **Error Handling**: Consistent error responses across all endpoints
- **Input Validation**: Request validation with proper error messages

## API Endpoints

### Authentication
- `POST /auth/register` - Register a new user
- `POST /auth/login` - Authenticate and get JWT token
- `POST /auth/logout` - Logout (client-side token invalidation)

### User Management (Authenticated)
- `GET /me` - Get current user profile
- `PUT /me` - Update current user profile  
- `POST /me/verify-email` - Verify user email address
- `POST /me/deposit` - Deposit funds to user account

### Order Management (Authenticated)
- `POST /orders` - Create a new order
- `GET /orders` - Get user's orders (with pagination and filtering)
- `GET /orders/{id}` - Get specific order details
- `PUT /orders/{id}` - Update an existing order
- `DELETE /orders/{id}` - Cancel an order

### System
- `GET /health` - Health check endpoint
- `GET /swagger-ui/` - Swagger UI interface

## Configuration

The API Gateway is configured via environment variables from the **root workspace `.env` file**:

```bash
# Server Configuration
API_GATEWAY_ADDRESS=0.0.0.0:8080

# Microservice URLs (matches your existing services)
USER_SERVICE_URL=http://localhost:3001
ORDER_SERVICE_URL=http://localhost:3002

# JWT Configuration
JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
JWT_EXPIRY_HOURS=24
```

**Note**: All configuration is centralized in the root `.env` file of the workspace.

## Authentication

All endpoints except `/health`, `/auth/*`, and `/swagger-ui/*` require authentication using JWT Bearer tokens:

```http
Authorization: Bearer <jwt_token>
```

## Pagination

List endpoints support pagination with the following query parameters:

- `page` (default: 1) - Page number
- `limit` (default: 20, max: 100) - Items per page
- `status` (optional) - Filter by order status
- `symbol` (optional) - Filter by symbol

Example:
```
GET /orders?page=1&limit=50&status=PENDING&symbol=AAPL
```

Response includes pagination metadata:
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 50,
    "total": 150,
    "pages": 3,
    "has_next": true,
    "has_prev": false
  }
}
```

## Error Handling

All endpoints return consistent error responses:

```json
{
  "error": "error_type",
  "message": "Human readable error message",
  "status": 400
}
```

Common HTTP status codes:
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `422` - Unprocessable Entity
- `500` - Internal Server Error
- `503` - Service Unavailable

## Running

```bash
# Ensure the root .env file is configured with:
# - USER_SERVICE_URL=http://localhost:3001
# - ORDER_SERVICE_URL=http://localhost:3002  
# - JWT_SECRET=your-secret-key
# - API_GATEWAY_ADDRESS=0.0.0.0:8080

# Run the API Gateway from the workspace root
cargo run -p api_gateway
```

The API will be available at `http://localhost:8080` by default.

## Development

### Adding New Endpoints

1. Add the handler function in `src/api.rs`
2. Add the route in `src/main.rs` 
3. Update the OpenAPI documentation in the handler
4. Add appropriate client calls in `src/clients.rs` if needed

### Testing

Access the Swagger UI at `http://localhost:8080/swagger-ui/` to explore and test the API interactively.

## Architecture

The API Gateway follows a layered architecture:

- **Handler Layer** (`src/api.rs`) - HTTP request handlers
- **Service Layer** (`src/clients.rs`) - Microservice clients
- **Middleware Layer** (`src/middleware.rs`) - Authentication, CORS, etc.
- **Model Layer** (`src/models.rs`) - Request/response models
- **Error Layer** (`src/error.rs`) - Error handling and responses

## Security Considerations

- JWT tokens are stateless and contain user ID
- All sensitive endpoints require authentication
- CORS is configurable for production environments
- Input validation prevents injection attacks
- Rate limiting can be added as middleware
- HTTPS should be used in production
