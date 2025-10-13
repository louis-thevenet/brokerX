# Webapp Market Data Integration

## Overview
The BrokerX webapp has been updated to display real-time market data from the new market-data-service via the API Gateway.

## Features Added

### 1. Market Data Section
A new dedicated section displays available trading symbols and their current prices with the following features:

- **Real-time Price Display**: Shows current prices for all available symbols
- **Price Change Indicators**: Visual indicators (▲/▼) and color coding for price movements
  - Green (↑) for price increases
  - Red (↓) for price decreases
  - Gray for no change
- **Auto-refresh**: Automatically refreshes every 30 seconds when logged in
- **Manual Refresh**: Button to manually refresh market data
- **Quick Buy**: One-click button to pre-fill order form with selected symbol
- **Price History**: View historical prices for any symbol

### 2. Enhanced Trading Form
The order placement form now includes:

- **Symbol Autocomplete**: Datalist showing all available symbols
- **Pre-filled Forms**: Quick buy feature pre-fills symbol and order type

### 3. API Endpoints Used

All endpoints accessed via API Gateway at `http://localhost:8080`:

- `GET /api/v1/market/symbols` - Get list of available symbols
  - Response: `{ symbols: ["AAPL", "GOOGL", ...], count: N }`

- `GET /api/v1/market/prices` - Get current prices for all symbols
  - Response: `{ prices: [{symbol, price, open, high, low, volume, timestamp}, ...], count: N }`

- `GET /api/v1/market/prices/{symbol}/history` - Get historical prices
  - Response: `{ symbol: "AAPL", prices: [{...}, ...], count: N }`

## UI Components

### Market Data Table
Located in the dashboard section, displays:

| Column | Description |
|--------|-------------|
| Symbol | Stock symbol (monospace font) |
| Current Price | Latest price with change indicator |
| 24h Change | Placeholder for future implementation |
| Volume | Placeholder for future implementation |
| Last Updated | Timestamp of last price update |
| Actions | Quick Buy and History buttons |

### Price Change Indicators

- **Color Coding**:
  - `#4caf50` (green) for price increases
  - `#f44336` (red) for price decreases
  - `#666` (gray) for no change

- **Symbols**:
  - `▲` for upward movement
  - `▼` for downward movement

### Auto-Refresh Mechanism

- Refreshes every 30 seconds when market data section is visible
- Automatically stops when user logs out
- Maintains price cache to detect and display price changes

## Files Modified

### /home/louis/src/brokerx/webapp/index.html

1. **Added Market Data Section** (before Trading Section):
   ```html
   <div id="market-data-section" class="section hidden">
       <h2>📊 Market Data</h2>
       <button onclick="loadMarketData()">🔄 Refresh Prices</button>
       <div id="market-prices-table"></div>
       <div id="market-message"></div>
   </div>
   ```

2. **Enhanced Trading Form**:
   - Added datalist for symbol autocomplete
   ```html
   <input type="text" id="order-symbol" list="available-symbols" required>
   <datalist id="available-symbols"></datalist>
   ```

3. **Added CSS Styles**:
   - `.price-up` - Green styling for price increases
   - `.price-down` - Red styling for price decreases
   - `.price-neutral` - Gray styling for no change
   - `.symbol-cell` - Monospace font for symbols
   - `.refresh-indicator` - Pulse animation for refresh status

### /home/louis/src/brokerx/webapp/app.js

1. **Added Market Data State**:
   ```javascript
   let marketDataCache = {};
   let marketDataRefreshInterval = null;
   ```

2. **New Functions**:
   - `loadMarketData()` - Fetch and display market data
   - `updateSymbolDatalist()` - Update order form autocomplete
   - `displayMarketPrices()` - Render price table with change indicators
   - `quickBuy()` - Pre-fill order form for quick trading
   - `viewHistory()` - Fetch historical prices
   - `displayPriceHistory()` - Render historical price table
   - `closeHistory()` - Close history view
   - `startMarketDataRefresh()` - Start auto-refresh timer
   - `stopMarketDataRefresh()` - Stop auto-refresh timer

3. **Updated Functions**:
   - `showDashboard()` - Now loads market data and starts auto-refresh
   - `showAuthSection()` - Stops market data refresh
   - `logout()` - Clears cache and stops refresh

## User Flow

1. **Login**: User logs in and is shown the dashboard
2. **Auto-load**: Market data automatically loads and displays
3. **Browse**: User sees real-time prices with change indicators
4. **Quick Trade**: User clicks "Buy" to pre-fill order form
5. **View History**: User can view historical prices for analysis
6. **Auto-refresh**: Prices update every 30 seconds
7. **Manual Refresh**: User can click refresh button anytime
8. **Place Order**: Symbol autocomplete helps user enter valid symbols

## Technical Details

### Price Change Detection
The webapp maintains a cache of previous prices to detect changes:
```javascript
const previousPrice = marketDataCache[symbol];
if (currentPrice > previousPrice) {
  // Show green with ▲
} else if (currentPrice < previousPrice) {
  // Show red with ▼
}
marketDataCache[symbol] = currentPrice;
```

### Auto-refresh Implementation
- Uses `setInterval` with 30-second interval
- Conditionally refreshes only if market section is visible
- Cleans up interval on logout or navigation away

### Error Handling
- Graceful fallback for JSON parsing errors
- User-friendly error messages
- Console logging for debugging
- Network error detection

## Future Enhancements

1. **Real-time Updates**: WebSocket support for instant price updates
2. **Charts**: Price charts using Chart.js or similar library
3. **Watchlist**: User-specific symbol watchlist
4. **Price Alerts**: Notifications when prices reach target levels
5. **24h Statistics**: Real change percentage and volume data
6. **Market Depth**: Order book visualization
7. **Search/Filter**: Search symbols by name or filter by sector
8. **Sorting**: Sort table by any column

## Testing

To test the market data integration:

1. Start all services (databases, market-data-service, API Gateway)
2. Initialize market data: `POST http://localhost:3001/api/v1/initialize`
3. Open webapp: `http://localhost:3000/webapp/`
4. Login with test account
5. Navigate to Market Data section
6. Verify prices are displayed
7. Test Quick Buy button
8. Test History view
9. Wait 30 seconds to verify auto-refresh
10. Check console for any errors

## Dependencies

- API Gateway must be running on `http://localhost:8080`
- Market Data Service must be running on `http://localhost:3001`
- PostgreSQL database for market data must be initialized
- Market data must be initialized via `/api/v1/initialize` endpoint
