# Market Data Service API Integration Guide

## 📋 Overview

This document outlines the recommended API integration strategy for the Market Data Service with the API Gateway and WebApp.

## 🎯 Recommended API Endpoints

### Public Endpoints (No Authentication Required)

These endpoints provide market data that should be publicly accessible:

#### 1. **GET /api/v1/market/prices/{symbol}**
- **Purpose**: Get current price for a specific symbol
- **Use Case**: Display current price on asset detail page
- **Response**: Single price with OHLCV data
- **Example**: `/api/v1/market/prices/AAPL`

#### 2. **GET /api/v1/market/prices**
- **Purpose**: Get current prices for multiple symbols
- **Query Params**: `symbols` (comma-separated list)
- **Use Case**: Display prices on dashboard, watchlist
- **Response**: Array of prices
- **Example**: `/api/v1/market/prices?symbols=AAPL,GOOGL,MSFT`

#### 3. **GET /api/v1/market/symbols**
- **Purpose**: Get list of available trading symbols
- **Use Case**: Populate symbol selector, search functionality
- **Response**: Array of available symbols
- **Example**: `/api/v1/market/symbols`

#### 4. **GET /api/v1/market/prices/{symbol}/history**
- **Purpose**: Get historical price data for charting
- **Query Params**: 
  - `start_date` (optional)
  - `end_date` (optional)
  - `limit` (default: 100)
- **Use Case**: Display price charts, technical analysis
- **Response**: Array of historical prices
- **Example**: `/api/v1/market/prices/AAPL/history?limit=30`

### Admin Endpoints (Authentication Required - Admin Only)

These endpoints should be restricted to administrative users:

#### 5. **POST /api/v1/admin/market/initialize**
- **Purpose**: Initialize market data with default symbols
- **Auth**: Required (Admin role)
- **Use Case**: System setup, data seeding
- **Response**: Count of symbols created

#### 6. **POST /api/v1/admin/market/simulate**
- **Purpose**: Manually trigger price simulation
- **Auth**: Required (Admin role)
- **Use Case**: Testing, manual price updates
- **Response**: Count of symbols updated

## 🏗️ Architecture Recommendations

### 1. **Client Separation**
```
API Gateway
├── UserClient      → User Service (port 3001)
├── OrderClient     → Order Service (port 3002)
└── MarketClient    → Market Data Service (port 3003)
```

### 2. **Caching Strategy**
- **In-Memory Cache**: Cache prices for 5-10 seconds to reduce load
- **Reason**: Prices update every 30 seconds, so short cache is acceptable
- **Implementation**: Use `tokio::sync::RwLock<HashMap<String, (Price, Instant)>>`

### 3. **WebSocket Consideration** (Future Enhancement)
- Real-time price updates via WebSocket
- Reduces polling overhead
- Better user experience for live trading

## 📱 WebApp Integration

### Dashboard View
```javascript
// Fetch multiple prices for dashboard
fetch('/api/v1/market/prices?symbols=AAPL,GOOGL,MSFT,AMZN,TSLA')
  .then(res => res.json())
  .then(data => updateDashboard(data.prices));
```

### Asset Detail Page
```javascript
// Fetch single asset with history
Promise.all([
  fetch('/api/v1/market/prices/AAPL'),
  fetch('/api/v1/market/prices/AAPL/history?limit=30')
]).then(([current, history]) => {
  // Display current price and chart
});
```

### Order Creation
```javascript
// Get current price before order placement
fetch('/api/v1/market/prices/AAPL')
  .then(res => res.json())
  .then(data => {
    // Pre-fill order form with current price
    document.getElementById('price').value = data.price;
  });
```

### Portfolio Holdings
```javascript
// Fetch prices for all holdings
const symbols = portfolio.holdings.map(h => h.symbol).join(',');
fetch(`/api/v1/market/prices?symbols=${symbols}`)
  .then(res => res.json())
  .then(data => calculatePortfolioValue(data.prices));
```

## 🔄 Data Flow

### Order Creation Flow
```
1. User selects symbol → Fetch current price
2. User enters quantity → Calculate total cost
3. Check user balance → Validate against current price
4. Create order → Use current market price
5. Update portfolio → Reflect new holding
```

### Portfolio Valuation Flow
```
1. Fetch user holdings → Get symbols and quantities
2. Fetch current prices → For all symbols
3. Calculate value → quantity × current_price
4. Display portfolio → With real-time values
```

## 🎨 UI Components Needed

### 1. **Price Ticker Component**
```javascript
<PriceTicker 
  symbol="AAPL" 
  showChange={true} 
  refreshInterval={5000} 
/>
```

### 2. **Price Chart Component**
```javascript
<PriceChart 
  symbol="AAPL" 
  period="1M" 
  type="candlestick" 
/>
```

### 3. **Market Overview Component**
```javascript
<MarketOverview 
  symbols={['AAPL', 'GOOGL', 'MSFT']} 
  layout="grid"
/>
```

### 4. **Symbol Search Component**
```javascript
<SymbolSearch 
  onSelect={(symbol) => navigate(`/trade/${symbol}`)}
  placeholder="Search stocks..."
/>
```

## 🔐 Security Considerations

### Public Endpoints
- **Rate Limiting**: 100 requests/minute per IP
- **No Authentication**: Market data is public
- **CORS**: Allow from WebApp origin

### Admin Endpoints
- **Authentication**: JWT token required
- **Authorization**: Admin role check
- **Audit Logging**: Log all admin actions

## 📊 Response Formats

### Current Price Response
```json
{
  "symbol": "AAPL",
  "price": 175.50,
  "open": 174.20,
  "high": 176.80,
  "low": 173.90,
  "volume": 1500000,
  "timestamp": "2024-10-13T10:30:00Z"
}
```

### Multiple Prices Response
```json
{
  "prices": [
    {
      "symbol": "AAPL",
      "price": 175.50,
      ...
    },
    {
      "symbol": "GOOGL",
      "price": 140.25,
      ...
    }
  ],
  "count": 2
}
```

### Historical Prices Response
```json
{
  "symbol": "AAPL",
  "prices": [
    {
      "price": 175.50,
      "timestamp": "2024-10-13T10:30:00Z",
      ...
    },
    ...
  ],
  "count": 30
}
```

## 🚀 Implementation Priority

### Phase 1: Basic Integration
1. ✅ Create MarketDataClient in API Gateway
2. ✅ Add public market data routes
3. ✅ Update WebApp to fetch prices
4. ✅ Display prices on order form

### Phase 2: Enhanced Features
5. Add price caching in API Gateway
6. Implement price charts in WebApp
7. Add market overview dashboard
8. Real-time portfolio valuation

### Phase 3: Advanced Features
9. WebSocket for real-time prices
10. Technical indicators
11. Price alerts
12. Historical analysis tools

## 🧪 Testing Strategy

### API Gateway Tests
- Test MarketDataClient methods
- Test route handlers
- Test error handling
- Test caching (if implemented)

### Integration Tests
- Test end-to-end price fetching
- Test order creation with current prices
- Test portfolio valuation accuracy

### WebApp Tests
- Test price display components
- Test chart rendering
- Test search functionality
- Test auto-refresh behavior

## 📝 Next Steps

1. **Implement MarketDataClient** in API Gateway
2. **Add routes** to API Gateway router
3. **Update WebApp** to consume new endpoints
4. **Add price displays** to existing pages
5. **Test integration** thoroughly
6. **Document** API in Swagger

---

**Recommended Immediate Actions:**
1. Create `MarketDataClient` in `api_gateway/src/clients.rs`
2. Add routes in `api_gateway/src/api.rs`
3. Update `api_gateway/src/models.rs` with market data models
4. Add market data section to Swagger docs
5. Update WebApp to display live prices
