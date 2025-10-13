# Order Service - Quick Reference

## 🚀 API Endpoints

### Order Service (Port 3002)
```
POST   /orders                 - Create new order
GET    /orders?client_id=&page=&limit=  - List orders
GET    /orders/{id}            - Get specific order
PUT    /orders/{id}            - Update order
DELETE /orders/{id}            - Cancel order
GET    /health                 - Health check
GET    /swagger-ui             - API Documentation
```

### API Gateway (Port 3000)
```
POST   /orders                 - Create new order (authenticated)
GET    /orders?page=&limit=    - List my orders (authenticated)
GET    /orders/{id}            - Get specific order (authenticated)
PUT    /orders/{id}            - Update order (authenticated)
DELETE /orders/{id}            - Cancel order (authenticated)
```

## 📦 Request/Response Examples

### Create Order
**Request:**
```json
POST /orders
{
  "symbol": "AAPL",
  "quantity": 10,
  "order_side": "Buy",
  "order_type": {
    "type": "limit",
    "price": 150.00
  }
}
```

**Response (201 Created):**
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "client_id": "user-uuid",
  "date": "2025-10-13T10:30:00Z",
  "symbol": "AAPL",
  "quantity": 10,
  "status": {"status": "pending"},
  "order_type": {"type": "limit", "price": 150.00},
  "order_side": "Buy",
  "filled_price": null
}
```

### List Orders
**Request:**
```
GET /orders?client_id=user-uuid&page=1&limit=20
```

**Response (200 OK):**
```json
{
  "data": [
    {
      "id": "order-uuid-1",
      "client_id": "user-uuid",
      "date": "2025-10-13T10:30:00Z",
      "symbol": "AAPL",
      "quantity": 10,
      "status": {"status": "pending"},
      "order_type": {"type": "market"},
      "order_side": "Buy",
      "filled_price": null
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total_items": 1,
    "total_pages": 1
  }
}
```

### Update Order
**Request:**
```json
PUT /orders/{id}
{
  "quantity": 20,
  "order_type": {
    "type": "limit",
    "price": 155.00
  }
}
```

**Response (200 OK):**
```json
{
  "id": "order-uuid",
  "client_id": "user-uuid",
  "quantity": 20,
  "order_type": {"type": "limit", "price": 155.00},
  ...
}
```

### Cancel Order
**Request:**
```
DELETE /orders/{id}
```

**Response (200 OK):**
```
(empty body)
```

## 🎯 Status Codes

| Code | Meaning | When |
|------|---------|------|
| 200  | OK | Successful GET, PUT, DELETE |
| 201  | Created | Successful POST |
| 400  | Bad Request | Invalid input data |
| 401  | Unauthorized | Missing/invalid JWT |
| 403  | Forbidden | Order belongs to another user |
| 404  | Not Found | Order or user not found |
| 409  | Conflict | Order cannot be modified/cancelled |
| 503  | Service Unavailable | Downstream service failure |

## 🔒 Business Rules

### Order Status Transitions
```
Pending → Filled       (via processing service)
Pending → Rejected     (via processing service)
Pending → Cancelled    (via API)
Pending → PartiallyFilled → Filled (via processing service)
```

### Modification Rules
- ✅ Can update: Pending, PartiallyFilled
- ❌ Cannot update: Filled, Rejected, Cancelled

### Cancellation Rules
- ✅ Can cancel: Pending, PartiallyFilled
- ❌ Cannot cancel: Filled, Rejected, Cancelled

## 🗄️ Database Schema (Simplified)

```sql
orders (
  id UUID PRIMARY KEY,
  client_id UUID NOT NULL,
  date TIMESTAMPTZ,
  symbol VARCHAR(20),
  quantity BIGINT CHECK (quantity > 0),
  order_side VARCHAR(10),
  status VARCHAR(30),
  filled_price DOUBLE PRECISION
)
```

## 🧪 Testing Commands

```bash
# Create order
curl -X POST http://localhost:3000/orders \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "AAPL",
    "quantity": 10,
    "order_side": "Buy",
    "order_type": {"type": "market"}
  }'

# List orders
curl http://localhost:3000/orders?page=1&limit=10 \
  -H "Authorization: Bearer $TOKEN"

# Get order
curl http://localhost:3000/orders/{ORDER_ID} \
  -H "Authorization: Bearer $TOKEN"

# Update order
curl -X PUT http://localhost:3000/orders/{ORDER_ID} \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"quantity": 20}'

# Cancel order
curl -X DELETE http://localhost:3000/orders/{ORDER_ID} \
  -H "Authorization: Bearer $TOKEN"
```

## 🏗️ Architecture

```
┌─────────┐      ┌──────────────┐      ┌──────────────┐
│ Client  │─────▶│ API Gateway  │─────▶│ Order Service│
└─────────┘      │  (Port 3000) │      │  (Port 3002) │
                 │  - JWT Auth  │      │  - CRUD Only │
                 │  - Routing   │      │  - Status: P │
                 └──────────────┘      └──────────────┘
                        │                       │
                        ▼                       ▼
                 ┌──────────────┐      ┌──────────────┐
                 │ User Service │      │  PostgreSQL  │
                 │  (Port 3001) │      │  (orders DB) │
                 └──────────────┘      └──────────────┘
```

## 📝 Notes

- All timestamps in UTC
- Order IDs are UUIDs
- Pagination: page starts at 1, default limit is 20, max is 100
- Orders created with status: Pending
- Future: Processing Service will update to Filled/Rejected
- Cancelled orders remain in database (soft delete)

## 🔗 Related Documentation

- Full Implementation: `/docs/ORDER_SERVICE_IMPLEMENTATION.md`
- API Documentation: `http://localhost:3002/swagger-ui`
- Architecture Overview: `/docs/arc42/`
