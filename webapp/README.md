# BrokerX Web Application

A simple web frontend that demonstrates the complete BrokerX trading platform functionality through the API Gateway.

## Features

### 🔐 **Authentication**
- User registration with initial balance
- Email/password login with JWT tokens
- Automatic token management and persistence

### 👤 **User Profile Management**
- View user profile and balance
- Update profile information (email, name)
- Email verification workflow
- Fund deposits to trading account

### 📈 **Trading Operations**
- Place market and limit orders
- Buy/sell different symbols (stocks)
- Real-time order validation
- Balance checks before order placement

### 📋 **Order Management**
- View all user orders with pagination
- Filter orders by status and symbol
- Cancel pending orders
- Order history with status tracking

### 💡 **User Experience**
- Clean, simple interface
- Real-time error handling and success messages
- Automatic data refresh after operations
- Responsive design for different screen sizes

## Getting Started

### Prerequisites
1. BrokerX API Gateway running on `http://localhost:8080`
2. User Service and Order Service running
3. PostgreSQL databases set up and running

### Running the Web App

#### Option 1: Simple HTTP Server (Python)
```bash
cd webapp
python3 -m http.server 3000
```

#### Option 2: Simple HTTP Server (Node.js)
```bash
cd webapp
npx http-server -p 3000
```

#### Option 3: Any web server
Just serve the files in the `webapp` directory on any HTTP server.

Then open: `http://localhost:3000`

### Complete Startup Sequence

1. **Start databases:**
   ```bash
   sudo ./start-databases.sh
   ```

2. **Start all services:**
   ```bash
   ./start-microservices.sh
   ```

3. **Start web app:**
   ```bash
   cd webapp && python3 -m http.server 3000
   ```

4. **Open browser:** `http://localhost:3000`

## Usage Flow

### 1. Register/Login
- New users: Click "Register" and fill in details
- Existing users: Login with email/password
- System automatically manages JWT tokens

### 2. Profile Management
- View balance and account info
- Update personal information
- Verify email if needed
- Add funds to your account

### 3. Trading
- Enter symbol (e.g., "AAPL", "GOOGL")
- Choose quantity and order type
- Select Buy/Sell
- For limit orders, set price
- System validates balance and places order

### 4. Order Management
- View all your orders
- Filter by status or symbol
- Cancel pending orders
- Navigate through pages for large order lists

## API Integration

The webapp demonstrates complete integration with the BrokerX API Gateway:

- **Authentication:** `/auth/login`, `/auth/register`, `/auth/logout`
- **User Management:** `/me` (GET/PUT), `/me/verify-email`, `/me/deposit`
- **Trading:** `/orders` (POST/GET), `/orders/{id}` (GET/DELETE)
- **System:** `/health` for API status

## Error Handling

- Network errors (API Gateway down)
- Authentication errors (invalid credentials, expired tokens)
- Validation errors (insufficient balance, invalid data)
- Business logic errors (order placement failures)

## Development Notes

### Configuration
- API base URL is set in `app.js`: `const API_BASE_URL = 'http://localhost:8080'`
- Change this if your API Gateway runs on a different port

### CORS
The API Gateway is configured with permissive CORS for development. In production, configure specific origins.

### Authentication
- JWT tokens are stored in `localStorage`
- Automatic token refresh is not implemented (tokens expire after 24h by default)
- Manual logout clears tokens

### Data Flow
1. User actions → JavaScript functions
2. API calls to Gateway → Microservices
3. Response handling → UI updates
4. Error handling → User feedback

## Testing the Complete System

### Registration Flow (UC-01)
1. Click "Register"
2. Fill form with email, password, name, initial balance
3. Submit → User created in database
4. Login with new credentials

### Fund Management (UC-03)
1. After login, see current balance
2. Use "Deposit Funds" to add money
3. Balance updates immediately

### Order Placement (UC-05)
1. Fill order form (symbol, quantity, type)
2. Submit → Order sent to Order Service
3. View in "My Orders" section
4. Cancel if still pending

### Complete User Journey
1. Register → Login → Deposit Funds → Place Order → View Orders → Cancel/Modify

This demonstrates the complete microservices architecture working together through a unified API Gateway.
