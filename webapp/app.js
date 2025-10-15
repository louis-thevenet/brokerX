// BrokerX Trading Platform - Professional Web Application
const API_BASE_URL = 'http://localhost:8080';
let currentToken = localStorage.getItem('brokerx_token');
let currentUser = null;

// MFA state management (mandatory for all users)
let pendingMfaChallenge = null;
let mfaStep = null; // 'registration' or 'login'
let isMfaSubmitting = false; // Prevent duplicate submissions

// Market data cache
let marketDataCache = {};
let marketDataRefreshInterval = null;

// Initialize app on load
document.addEventListener('DOMContentLoaded', function() {
  if (currentToken) {
    loadUserProfile();
    showDashboard();
  } else {
    showAuthSection();
  }
  setupMfaAutoSubmit();
});

// Utility Functions
function showMessage(elementId, message, isError = false) {
  const element = document.getElementById(elementId);
  element.innerHTML = `<p class="${isError ? 'error' : 'success'}">${message}</p>`;
  setTimeout(() => element.innerHTML = '', 5000);
}

function makeApiCall(endpoint, options = {}) {
  const defaultOptions = {
    headers: {
      'Content-Type': 'application/json',
    }
  };

  if (currentToken) {
    defaultOptions.headers['Authorization'] = `Bearer ${currentToken}`;
  }

  const finalOptions = {
    ...defaultOptions,
    ...options,
    headers: { ...defaultOptions.headers, ...options.headers }
  };

  return fetch(`${API_BASE_URL}${endpoint}`, finalOptions);
}

// Authentication Functions
async function login(event) {
  event.preventDefault();

  const email = document.getElementById('login-email').value;
  const password = document.getElementById('login-password').value;

  try {
    const response = await makeApiCall('/v1/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    });

    if (response.ok) {
      const data = await response.json();
      
      // All logins require MFA - always show verification screen
      pendingMfaChallenge = data.challenge_id;
      mfaStep = 'login';
      sessionStorage.setItem('pending_user_id', data.user_id);
      showMfaVerification('Multi-Factor Authentication', 
        'Please enter the verification code sent to your email to complete login. Check your email for the 6-digit code.');
    } else {
      const error = await response.json();
      showMessage('auth-message', error.message || 'Login failed', true);
    }
  } catch (err) {
    showMessage('auth-message', 'Network error occurred', true);
    console.error('Login error:', err);
  }
}

async function register(event) {
  event.preventDefault();

  const email = document.getElementById('reg-email').value;
  const password = document.getElementById('reg-password').value;
  const firstname = document.getElementById('reg-firstname').value;
  const surname = document.getElementById('reg-surname').value;

  try {
    const response = await makeApiCall('/v1/auth/register', {
      method: 'POST',
      body: JSON.stringify({
        email,
        password,
        firstname,
        surname,
      })
    });

    if (response.ok) {
      const data = await response.json();
      
      // Registration creates the account - MFA challenge will be sent during login
      showMessage('auth-message', 
        'Registration successful! Please login to verify your account. You will receive a verification code via email during login.');
      
      // Clear registration form and show login
      document.getElementById('reg-email').value = '';
      document.getElementById('reg-password').value = '';
      document.getElementById('reg-firstname').value = '';
      document.getElementById('reg-surname').value = '';
      showLogin();
    } else {
      const error = await response.json();
      showMessage('auth-message', error.message || 'Registration failed', true);
    }
  } catch (err) {
    showMessage('auth-message', 'Network error occurred', true);
    console.error('Registration error:', err);
  }
}

function logout() {
  currentToken = null;
  currentUser = null;
  localStorage.removeItem('brokerx_token');
  stopMarketDataRefresh();
  marketDataCache = {};
  showAuthSection();
  showMessage('auth-message', 'Logged out successfully');
}

// User Profile Functions
async function loadUserProfile() {
  try {
    const response = await makeApiCall('/v1/me');

    if (response.ok) {
      currentUser = await response.json();
      console.log('User profile loaded:', {
        id: currentUser.id,
        email: currentUser.email,
        balance: currentUser.balance,
        holdingsCount: currentUser.holdings ? Object.keys(currentUser.holdings).length : 0,
        holdings: currentUser.holdings
      });
      updateUserInfo();
    } else if (response.status === 401) {
      // Token expired
      logout();
    } else {
      showMessage('profile-message', 'Failed to load profile', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Profile load error:', err);
  }
}

function updateUserInfo() {
  if (!currentUser) return;

  document.getElementById('user-info').innerHTML = `
        <h3>Welcome, ${currentUser.firstname} ${currentUser.surname}!</h3>
        <p><strong>Email:</strong> ${currentUser.email} ${currentUser.is_verified ? '✅' : '❌ (Not Verified)'}</p>
        <p><strong>Balance:</strong> $${currentUser.balance.toFixed(2)}</p>
        <p><strong>Member since:</strong> ${new Date(currentUser.created_at).toLocaleDateString()}</p>
    `;
}

async function updateProfile(event) {
  event.preventDefault();

  const email = document.getElementById('update-email').value;
  const firstname = document.getElementById('update-firstname').value;
  const surname = document.getElementById('update-surname').value;

  const updateData = {};
  if (email) updateData.email = email;
  if (firstname) updateData.firstname = firstname;
  if (surname) updateData.surname = surname;

  if (Object.keys(updateData).length === 0) {
    showMessage('profile-message', 'Please fill at least one field to update', true);
    return;
  }

  try {
    const response = await makeApiCall('/v1/me', {
      method: 'PUT',
      body: JSON.stringify(updateData)
    });

    if (response.ok) {
      showMessage('profile-message', 'Profile updated successfully!');
      await loadUserProfile();
      // Clear form
      document.getElementById('update-email').value = '';
      document.getElementById('update-firstname').value = '';
      document.getElementById('update-surname').value = '';
    } else {
      const error = await response.json();
      showMessage('profile-message', error.message || 'Update failed', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Update error:', err);
  }
}

async function depositFunds(event) {
  event.preventDefault();

  const amount = parseFloat(document.getElementById('deposit-amount').value);

  try {
    const response = await makeApiCall('/v1/me/deposit', {
      method: 'POST',
      body: JSON.stringify({ amount })
    });

    if (response.ok) {
      showMessage('profile-message', `Successfully deposited $${amount.toFixed(2)}!`);
      await loadUserProfile();
      document.getElementById('deposit-amount').value = '';
    } else {
      const error = await response.json();
      showMessage('profile-message', error.message || 'Deposit failed', true);
    }
  } catch (err) {
    showMessage('profile-message', 'Network error occurred', true);
    console.error('Deposit error:', err);
  }
}

// ============================================================================
// Holdings Functions
// ============================================================================

async function refreshHoldings() {
  // Reload user profile to get latest holdings, then reload market data, then display
  await loadUserProfile();
  await loadMarketData();
  loadHoldings();
}

async function loadHoldings() {
  if (!currentUser || !currentUser.id) {
    showMessage('holdings-message', 'User not loaded', true);
    return;
  }

  // Holdings are already in the currentUser object
  if (currentUser.holdings) {
    // Convert holdings object to array
    const holdingsArray = Object.values(currentUser.holdings);
    console.log('Holdings loaded from user profile:', holdingsArray);
    displayHoldings(holdingsArray);
  } else {
    // No holdings
    console.log('No holdings found in user profile');
    displayHoldings([]);
  }
}

function displayHoldings(holdings) {
  const summaryDiv = document.getElementById('holdings-summary');
  const tableDiv = document.getElementById('holdings-table');

  console.log('Displaying holdings:', holdings);
  console.log('Market data cache:', marketDataCache);

  if (!holdings || holdings.length === 0) {
    summaryDiv.innerHTML = '<p style="text-align: center; color: #666;">No holdings yet. Start trading to build your portfolio!</p>';
    tableDiv.innerHTML = '';
    return;
  }

  // Calculate portfolio summary
  let totalValue = 0;
  let totalCost = 0;
  let totalShares = 0;

  holdings.forEach(holding => {
    // Use market price if available, otherwise use average cost (prevents -100% on new holdings)
    const currentPrice = marketDataCache[holding.symbol] || holding.average_cost;
    console.log(`Holding ${holding.symbol}: quantity=${holding.quantity}, avg_cost=${holding.average_cost}, current_price=${currentPrice}`);
    const value = holding.quantity * currentPrice;
    const cost = holding.quantity * holding.average_cost;
    totalValue += value;
    totalCost += cost;
    totalShares += holding.quantity;
  });

  const totalProfit = totalValue - totalCost;
  const profitPercentage = totalCost > 0 ? (totalProfit / totalCost * 100) : 0;

  // Display summary cards
  summaryDiv.innerHTML = `
    <div class="holdings-summary">
      <div class="summary-card">
        <h4>Total Value</h4>
        <p class="value">$${totalValue.toFixed(2)}</p>
      </div>
      <div class="summary-card">
        <h4>Total Cost</h4>
        <p class="value">$${totalCost.toFixed(2)}</p>
      </div>
      <div class="summary-card">
        <h4>Profit/Loss</h4>
        <p class="value ${totalProfit >= 0 ? 'profit-positive' : 'profit-negative'}">
          ${totalProfit >= 0 ? '+' : ''}$${totalProfit.toFixed(2)}
          <span style="font-size: 0.8rem;">(${profitPercentage >= 0 ? '+' : ''}${profitPercentage.toFixed(2)}%)</span>
        </p>
      </div>
      <div class="summary-card">
        <h4>Total Shares</h4>
        <p class="value">${totalShares}</p>
      </div>
    </div>
  `;

  // Display holdings table
  let tableHTML = `
    <table>
      <thead>
        <tr>
          <th>Symbol</th>
          <th>Quantity</th>
          <th>Avg Cost</th>
          <th>Current Price</th>
          <th>Market Value</th>
          <th>Total Cost</th>
          <th>Profit/Loss</th>
          <th>Return %</th>
        </tr>
      </thead>
      <tbody>
  `;

  holdings.forEach(holding => {
    // Use market price if available, otherwise use average cost (prevents -100% on new holdings)
    const currentPrice = marketDataCache[holding.symbol] || holding.average_cost;
    const marketValue = holding.quantity * currentPrice;
    const totalCost = holding.quantity * holding.average_cost;
    const profitLoss = marketValue - totalCost;
    const returnPct = totalCost > 0 ? (profitLoss / totalCost * 100) : 0;

    const profitClass = profitLoss >= 0 ? 'profit-positive' : 'profit-negative';

    tableHTML += `
      <tr>
        <td class="symbol-cell">${holding.symbol}</td>
        <td>${holding.quantity}</td>
        <td>$${holding.average_cost.toFixed(2)}</td>
        <td class="${currentPrice > holding.average_cost ? 'price-up' : currentPrice < holding.average_cost ? 'price-down' : 'price-neutral'}">
          $${currentPrice.toFixed(2)}
        </td>
        <td>$${marketValue.toFixed(2)}</td>
        <td>$${totalCost.toFixed(2)}</td>
        <td class="${profitClass}">
          ${profitLoss >= 0 ? '+' : ''}$${profitLoss.toFixed(2)}
        </td>
        <td class="${profitClass}">
          ${returnPct >= 0 ? '+' : ''}${returnPct.toFixed(2)}%
        </td>
      </tr>
    `;
  });

  tableHTML += `
      </tbody>
    </table>
  `;

  tableDiv.innerHTML = tableHTML;
}

// ============================================================================
// Trading Functions - Order Placement
// ============================================================================
// The order API uses a RESTful structure with tagged enums for order_type and status:
// - order_type: { type: "market" } or { type: "limit", price: 150.0 }
// - status: { status: "pending" } or { status: "filled", date: "..." }
// ============================================================================

async function placeOrder(event) {
  event.preventDefault();

  const symbol = document.getElementById('order-symbol').value.toUpperCase();
  const quantity = parseInt(document.getElementById('order-quantity').value);
  const order_side = document.getElementById('order-side').value.toLowerCase(); // Convert to lowercase for API
  const order_type_value = document.getElementById('order-type').value;
  const price = document.getElementById('order-price').value;

  // Build order_type according to REST API format (tagged enum)
  // Note: The API Gateway expects OrderType as a tagged enum where Limit variant contains the price
  let order_type;
  if (order_type_value === 'MARKET') {
    order_type = { type: "market" };
  } else if (order_type_value === 'LIMIT' && price) {
    order_type = { type: "limit", price: parseFloat(price) };
  } else {
    showMessage('trading-message', 'Invalid order type or missing price', true);
    return;
  }

  const orderData = {
    symbol,
    quantity,
    order_side,
    order_type,
    price: order_type_value === 'LIMIT' ? parseFloat(price) : null
  };

  try {
    console.log('Placing order:', orderData);
    const response = await makeApiCall('/v1/orders', {
      method: 'POST',
      body: JSON.stringify(orderData)
    });

    console.log('Order response status:', response.status);
    console.log('Order response headers:', response.headers);
    
    if (response.ok) {
      const order = await response.json();
      console.log('Order created:', order);
      showMessage('trading-message', `Order placed successfully! Order ID: ${order.id}`);
      // Clear form
      document.getElementById('order-symbol').value = '';
      document.getElementById('order-quantity').value = '';
      document.getElementById('order-side').value = '';
      document.getElementById('order-type').value = '';
      document.getElementById('order-price').value = '';
      document.getElementById('order-price').classList.add('hidden');

      // Refresh orders, profile, and holdings
      await loadUserProfile(); // Load profile first to update currentUser with new holdings
      await loadMarketData(); // Load market data to get current prices
      if (!document.getElementById('orders-section').classList.contains('hidden')) {
        await loadOrders();
      }
      if (!document.getElementById('holdings-section').classList.contains('hidden')) {
        loadHoldings(); // Display holdings with updated data
      }
    } else {
      const responseText = await response.text();
      console.error('Order placement failed. Status:', response.status, 'Response:', responseText);
      try {
        const error = JSON.parse(responseText);
        showMessage('trading-message', error.message || error.error || 'Order placement failed', true);
      } catch (parseErr) {
        showMessage('trading-message', `Order placement failed (${response.status}): ${responseText}`, true);
      }
    }
  } catch (err) {
    showMessage('trading-message', 'Network error occurred', true);
    console.error('Order placement error:', err);
    console.error('Error stack:', err.stack);
  }
}

function togglePriceField() {
  const orderType = document.getElementById('order-type').value;
  const priceField = document.getElementById('order-price');

  if (orderType === 'LIMIT') {
    priceField.classList.remove('hidden');
    priceField.required = true;
  } else {
    priceField.classList.add('hidden');
    priceField.required = false;
    priceField.value = '';
  }
}

// ============================================================================
// Orders Functions - List, Update, Cancel
// ============================================================================
// The GET /orders endpoint returns paginated results:
// { data: [...], pagination: { page, limit, total_items, total_pages } }
// ============================================================================

async function loadOrders() {
  try {
    // REST API uses pagination - get first page with reasonable limit
    const response = await makeApiCall('/v1/orders?page=1&limit=50');

    if (response.ok) {
      const data = await response.json();
      // REST API returns paginated response: { data: [...], pagination: {...} }
      const orders = data.data || [];
      displayOrders(orders);
      
      // Show pagination info if available
      if (data.pagination) {
        const paginationInfo = `Showing page ${data.pagination.page} of ${data.pagination.total_pages} (${data.pagination.total_items} total orders)`;
        document.getElementById('orders-message').innerHTML = `<p style="color: #666; font-size: 0.9rem;">${paginationInfo}</p>`;
      }
    } else {
      const error = await response.json();
      showMessage('orders-message', error.message || error.error || 'Failed to load orders', true);
    }
  } catch (err) {
    showMessage('orders-message', 'Network error occurred', true);
    console.error('Orders load error:', err);
  }
}

function formatOrderType(orderType) {
  // orderType is a tagged enum: { type: "market" } or { type: "limit", price: 150.0 }
  if (!orderType) return 'Unknown';
  
  if (typeof orderType === 'string') {
    // Legacy format compatibility (shouldn't happen with new API)
    return orderType;
  }
  
  if (orderType.type === 'market') {
    return 'Market';
  } else if (orderType.type === 'limit') {
    return `Limit ($${orderType.price.toFixed(2)})`;
  }
  return 'Unknown';
}

function formatOrderStatus(status) {
  // status is a tagged enum: { status: "pending" } or { status: "filled", date: "..." }
  if (!status) return 'Unknown';
  
  // Debug: log the actual status object
  console.log('Formatting status:', status);
  
  if (typeof status === 'string') {
    // Legacy format compatibility (shouldn't happen with new API)
    return status;
  }
  
  if (status.status === 'pending') {
    return 'Pending';
  } else if (status.status === 'filled') {
    const date = new Date(status.date).toLocaleString();
    return `Filled (${date})`;
  } else if (status.status === 'cancelled') {
    const date = new Date(status.date).toLocaleString();
    return `Cancelled (${date})`;
  } else if (status.status === 'rejected') {
    const date = new Date(status.date).toLocaleString();
    return `Rejected (${date})`;
  } else if (status.status === 'partially_filled') {
    return `Partially Filled (${status.filled_quantity})`;
  }
  console.warn('Unknown status format:', status);
  return 'Unknown';
}

function canCancelOrder(status) {
  // Can only cancel pending or partially filled orders
  if (!status) return false;
  
  if (typeof status === 'string') {
    return status === 'PENDING';
  }
  
  return status.status === 'pending' || status.status === 'partially_filled';
}

function canUpdateOrder(status) {
  // Can only update pending or partially filled orders
  if (!status) return false;
  
  if (typeof status === 'string') {
    return status === 'PENDING';
  }
  
  return status.status === 'pending' || status.status === 'partially_filled';
}

function displayOrders(orders) {
  const tableContainer = document.getElementById('orders-table');

  if (!orders || orders.length === 0) {
    tableContainer.innerHTML = '<p>No orders found.</p>';
    return;
  }

  let html = `
    <table>
      <thead>
        <tr>
          <th>Date</th>
          <th>Symbol</th>
          <th>Side</th>
          <th>Type</th>
          <th>Quantity</th>
          <th>Filled Price</th>
          <th>Status</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
  `;

  orders.forEach(order => {
    const orderDate = new Date(order.date).toLocaleString();
    const filledPrice = order.filled_price ? `$${order.filled_price.toFixed(2)}` : '-';
    // Escape order data for HTML attributes
    const orderTypeJson = JSON.stringify(order.order_type).replace(/"/g, '&quot;');
    
    html += `
      <tr>
        <td style="font-size: 0.85rem;">${orderDate}</td>
        <td class="symbol-cell">${order.symbol}</td>
        <td style="text-transform: capitalize;">${order.order_side}</td>
        <td>${formatOrderType(order.order_type)}</td>
        <td>${order.quantity}</td>
        <td>${filledPrice}</td>
        <td>${formatOrderStatus(order.status)}</td>
        <td>
          ${canCancelOrder(order.status) ? 
            `<button onclick="cancelOrder('${order.id}')" class="btn-secondary" style="font-size: 0.85rem; padding: 4px 8px;">❌ Cancel</button>` : 
            '-'
          }
          ${canUpdateOrder(order.status) ? 
            `<button onclick="updateOrderDialog('${order.id}', ${order.quantity}, '${orderTypeJson}')" class="btn-info" style="font-size: 0.85rem; padding: 4px 8px; margin-left: 4px;">✏️ Edit</button>` : 
            ''
          }
        </td>
      </tr>
    `;
  });

  html += '</tbody></table>';
  tableContainer.innerHTML = html;
}



async function cancelOrder(orderId) {
  if (!confirm('Are you sure you want to cancel this order?')) {
    return;
  }

  try {
    const response = await makeApiCall(`/v1/orders/${orderId}`, {
      method: 'DELETE'
    });

    if (response.ok) {
      showMessage('orders-message', 'Order cancelled successfully!');
      await loadOrders();
      await loadUserProfile();
    } else {
      const error = await response.json();
      showMessage('orders-message', error.message || 'Failed to cancel order', true);
    }
  } catch (err) {
    showMessage('orders-message', 'Network error occurred', true);
    console.error('Cancel order error:', err);
  }
}

function updateOrderDialog(orderId, currentQuantity, currentTypeJson) {
  // Parse the order type from the escaped JSON string
  let currentType;
  try {
    currentType = JSON.parse(currentTypeJson.replace(/&quot;/g, '"'));
  } catch (e) {
    console.error('Failed to parse order type:', e);
    currentType = { type: 'market' };
  }

  // Show modal
  document.getElementById('update-order-modal').style.display = 'block';
  document.getElementById('update-order-id').value = orderId;
  document.getElementById('update-order-quantity').value = currentQuantity;
  
  // Set order type
  if (currentType.type === 'market') {
    document.getElementById('update-order-type').value = 'MARKET';
    document.getElementById('update-price-field').classList.add('hidden');
    document.getElementById('update-order-price').value = '';
  } else if (currentType.type === 'limit') {
    document.getElementById('update-order-type').value = 'LIMIT';
    document.getElementById('update-price-field').classList.remove('hidden');
    document.getElementById('update-order-price').value = currentType.price || '';
  }
  
  document.getElementById('update-order-message').innerHTML = '';
}

function closeUpdateOrderModal() {
  document.getElementById('update-order-modal').style.display = 'none';
}

function toggleUpdatePriceField() {
  const orderType = document.getElementById('update-order-type').value;
  const priceField = document.getElementById('update-price-field');
  const priceInput = document.getElementById('update-order-price');

  if (orderType === 'LIMIT') {
    priceField.classList.remove('hidden');
    priceInput.required = true;
  } else {
    priceField.classList.add('hidden');
    priceInput.required = false;
    priceInput.value = '';
  }
}

async function submitOrderUpdateFromModal(event) {
  event.preventDefault();
  
  const orderId = document.getElementById('update-order-id').value;
  const quantity = parseInt(document.getElementById('update-order-quantity').value);
  const orderTypeValue = document.getElementById('update-order-type').value;
  const price = document.getElementById('update-order-price').value;

  // Build order_type according to new API format
  let order_type;
  if (orderTypeValue === 'MARKET') {
    order_type = { type: "market" };
  } else if (orderTypeValue === 'LIMIT' && price) {
    order_type = { type: "limit", price: parseFloat(price) };
  } else {
    showMessage('update-order-message', 'Invalid order type or missing price', true);
    return;
  }

  const updateData = {
    quantity,
    order_type
  };

  try {
    const response = await makeApiCall(`/v1/orders/${orderId}`, {
      method: 'PUT',
      body: JSON.stringify(updateData)
    });

    if (response.ok) {
      showMessage('orders-message', 'Order updated successfully!');
      closeUpdateOrderModal();
      await loadOrders();
      await loadUserProfile();
    } else {
      const error = await response.json();
      showMessage('update-order-message', error.message || 'Failed to update order', true);
    }
  } catch (err) {
    showMessage('update-order-message', 'Network error occurred', true);
    console.error('Update order error:', err);
  }
}

// Market Data Functions
async function loadMarketData() {
  try {
    // First, load available symbols
    const symbolsResponse = await makeApiCall('/v1/market/symbols');
    
    if (!symbolsResponse.ok) {
      const error = await symbolsResponse.json().catch(() => ({ message: 'Failed to load market data' }));
      showMessage('market-message', error.message || 'Failed to load market data', true);
      return;
    }

    const symbolsData = await symbolsResponse.json();
    console.log('Symbols response:', symbolsData);
    
    // The API returns { symbols: [...], count: N }
    const symbols = symbolsData.symbols || symbolsData.data || [];
    
    if (!symbols || symbols.length === 0) {
      document.getElementById('market-prices-table').innerHTML = '<p>No market data available.</p>';
      return;
    }

    console.log('Available symbols:', symbols);
    
    // Update the datalist for order form
    updateSymbolDatalist(symbols);

    // Fetch prices for all symbols
    const pricesResponse = await makeApiCall('/v1/market/prices');
    
    if (pricesResponse.ok) {
      const pricesData = await pricesResponse.json();
      console.log('Prices response:', pricesData);
      
      // The API returns { prices: [...] }
      const prices = pricesData.prices || pricesData.data || [];
      console.log('Extracted prices array:', prices);
      
      displayMarketPrices(prices);
    } else {
      showMessage('market-message', 'Failed to load current prices', true);
    }
  } catch (err) {
    showMessage('market-message', 'Network error occurred', true);
    console.error('Market data load error:', err);
  }
}

function updateSymbolDatalist(symbols) {
  const datalist = document.getElementById('available-symbols');
  if (!datalist) return;
  
  datalist.innerHTML = symbols.map(symbol => `<option value="${symbol}">`).join('');
}

function displayMarketPrices(prices) {
  const tableContainer = document.getElementById('market-prices-table');

  if (!prices || prices.length === 0) {
    tableContainer.innerHTML = '<p>No prices available.</p>';
    return;
  }

  // Debug: Log the first price object to see its structure
  console.log('Sample price data:', prices[0]);

  let html = `
    <table>
      <thead>
        <tr>
          <th>Symbol</th>
          <th>Current Price</th>
          <th>Open</th>
          <th>High/Low</th>
          <th>Volume</th>
          <th>Last Updated</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
  `;

  prices.forEach(priceData => {
    // Ensure we have a symbol
    if (!priceData.symbol) {
      console.warn('Price data missing symbol:', priceData);
      return;
    }

    // Calculate price change if we have previous data
    const previousPrice = marketDataCache[priceData.symbol];
    let changeClass = 'price-neutral';
    let changeIndicator = '';
    
    if (previousPrice) {
      if (priceData.price > previousPrice) {
        changeClass = 'price-up';
        changeIndicator = '▲';
      } else if (priceData.price < previousPrice) {
        changeClass = 'price-down';
        changeIndicator = '▼';
      }
    }
    
    // Update cache
    marketDataCache[priceData.symbol] = priceData.price;
    
    // Format timestamp
    const lastUpdate = new Date(priceData.timestamp || Date.now()).toLocaleTimeString();
    
    // Format high/low
    const highLow = `$${priceData.high.toFixed(2)} / $${priceData.low.toFixed(2)}`;
    
    html += `
      <tr>
        <td class="symbol-cell">${priceData.symbol}</td>
        <td class="${changeClass}">$${priceData.price.toFixed(2)} ${changeIndicator}</td>
        <td>$${priceData.open.toFixed(2)}</td>
        <td>${highLow}</td>
        <td>${priceData.volume.toLocaleString()}</td>
        <td>${lastUpdate}</td>
        <td>
          <button onclick="quickBuy('${priceData.symbol}', ${priceData.price})" class="btn-primary" style="font-size: 0.85rem; padding: 4px 8px;">📈 Buy</button>
          <button onclick="viewHistory('${priceData.symbol}')" class="btn-info" style="font-size: 0.85rem; padding: 4px 8px;">📊 Chart</button>
        </td>
      </tr>
    `;
  });

  html += '</tbody></table>';
  tableContainer.innerHTML = html;
  
  // Refresh holdings display if holdings section is visible
  if (currentUser && !document.getElementById('holdings-section').classList.contains('hidden')) {
    // Just re-render with updated cache prices
    const holdingsTable = document.getElementById('holdings-table');
    if (holdingsTable && holdingsTable.innerHTML) {
      // Holdings are already loaded, just refresh the display
      loadHoldings();
    }
  }
}

function quickBuy(symbol, price) {
  // Scroll to trading section and pre-fill the form
  document.getElementById('order-symbol').value = symbol;
  document.getElementById('order-side').value = 'buy'; // lowercase for API
  document.getElementById('order-type').value = 'MARKET';
  document.getElementById('order-quantity').focus();
  
  // Scroll to trading section
  document.getElementById('trading-section').scrollIntoView({ behavior: 'smooth' });
  showMessage('trading-message', `Ready to buy ${symbol} at market price ($${price.toFixed(2)})`);
}

async function viewHistory(symbol) {
  try {
    const response = await makeApiCall(`/v1/market/prices/${symbol}/history`);
    
    if (response.ok) {
      const data = await response.json();
      // The API returns { symbol: String, prices: [...], count: N }
      const history = data.prices || [];
      showPriceChart(symbol, history);
    } else {
      const error = await response.json().catch(() => ({ message: `Failed to load history for ${symbol}` }));
      showMessage('market-message', error.message || `Failed to load history for ${symbol}`, true);
    }
  } catch (err) {
    showMessage('market-message', 'Network error occurred', true);
    console.error('Price history error:', err);
  }
}

let priceChart = null;

function showPriceChart(symbol, history) {
  if (!history || history.length === 0) {
    showMessage('market-message', `No price history available for ${symbol}`, true);
    return;
  }

  // Sort by timestamp (oldest first for chart)
  const sortedHistory = [...history].sort((a, b) => 
    new Date(a.timestamp) - new Date(b.timestamp)
  );

  // Prepare data
  const labels = sortedHistory.map(entry => 
    new Date(entry.timestamp).toLocaleString('en-US', { 
      month: 'short', 
      day: 'numeric', 
      hour: '2-digit', 
      minute: '2-digit' 
    })
  );
  
  const prices = sortedHistory.map(entry => entry.price);
  const volumes = sortedHistory.map(entry => entry.volume);
  
  // Calculate statistics
  const currentPrice = sortedHistory[sortedHistory.length - 1].price;
  const firstPrice = sortedHistory[0].price;
  const change = currentPrice - firstPrice;
  const changePercent = ((change / firstPrice) * 100).toFixed(2);
  const maxPrice = Math.max(...prices);
  const minPrice = Math.min(...prices);
  const avgPrice = (prices.reduce((a, b) => a + b, 0) / prices.length).toFixed(2);
  
  // Show modal
  document.getElementById('chart-modal').style.display = 'block';
  document.getElementById('chart-title').textContent = `${symbol} - Price History`;
  
  // Display statistics
  document.getElementById('chart-stats').innerHTML = `
    <div class="section" style="padding: 10px;">
      <strong>Current Price</strong><br>
      <span style="font-size: 1.5rem; color: ${change >= 0 ? '#4caf50' : '#f44336'}">
        $${currentPrice.toFixed(2)}
      </span>
    </div>
    <div class="section" style="padding: 10px;">
      <strong>Change</strong><br>
      <span style="font-size: 1.2rem; color: ${change >= 0 ? '#4caf50' : '#f44336'}">
        ${change >= 0 ? '+' : ''}$${change.toFixed(2)} (${changePercent}%)
      </span>
    </div>
    <div class="section" style="padding: 10px;">
      <strong>High / Low</strong><br>
      <span style="font-size: 1.2rem;">
        $${maxPrice.toFixed(2)} / $${minPrice.toFixed(2)}
      </span>
    </div>
    <div class="section" style="padding: 10px;">
      <strong>Average</strong><br>
      <span style="font-size: 1.2rem;">
        $${avgPrice}
      </span>
    </div>
  `;
  
  // Destroy previous chart if exists
  if (priceChart) {
    priceChart.destroy();
  }
  
  // Create new chart
  const ctx = document.getElementById('price-chart').getContext('2d');
  priceChart = new Chart(ctx, {
    type: 'line',
    data: {
      labels: labels,
      datasets: [{
        label: 'Price ($)',
        data: prices,
        borderColor: '#007cba',
        backgroundColor: 'rgba(0, 124, 186, 0.1)',
        borderWidth: 2,
        fill: true,
        tension: 0.4,
        pointRadius: 3,
        pointHoverRadius: 6,
        pointBackgroundColor: '#007cba',
        pointBorderColor: '#fff',
        pointBorderWidth: 2
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        intersect: false,
        mode: 'index'
      },
      plugins: {
        legend: {
          display: true,
          position: 'top'
        },
        tooltip: {
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          padding: 12,
          titleFont: { size: 14, weight: 'bold' },
          bodyFont: { size: 13 },
          callbacks: {
            label: function(context) {
              const price = context.parsed.y;
              const volume = volumes[context.dataIndex];
              return [
                `Price: $${price.toFixed(2)}`,
                `Volume: ${volume.toLocaleString()}`
              ];
            }
          }
        }
      },
      scales: {
        x: {
          grid: {
            display: false
          },
          ticks: {
            maxRotation: 45,
            minRotation: 45
          }
        },
        y: {
          beginAtZero: false,
          grid: {
            color: 'rgba(0, 0, 0, 0.05)'
          },
          ticks: {
            callback: function(value) {
              return '$' + value.toFixed(2);
            }
          }
        }
      }
    }
  });
}

function closeChartModal() {
  document.getElementById('chart-modal').style.display = 'none';
  if (priceChart) {
    priceChart.destroy();
    priceChart = null;
  }
}

// Close modal when clicking outside
window.onclick = function(event) {
  const chartModal = document.getElementById('chart-modal');
  const updateModal = document.getElementById('update-order-modal');
  
  if (event.target === chartModal) {
    closeChartModal();
  }
  
  if (event.target === updateModal) {
    closeUpdateOrderModal();
  }
}

function displayPriceHistory(symbol, history) {
  // Legacy function - now using showPriceChart instead
  showPriceChart(symbol, history);
}

function closeHistory() {
  closeChartModal();
}

function startMarketDataRefresh() {
  // Refresh market data every 30 seconds
  if (marketDataRefreshInterval) {
    clearInterval(marketDataRefreshInterval);
  }
  
  marketDataRefreshInterval = setInterval(async () => {
    if (!document.getElementById('market-data-section').classList.contains('hidden')) {
      await loadMarketData();
      // Refresh holdings after market data loads to update with new prices
      if (!document.getElementById('holdings-section').classList.contains('hidden')) {
        loadHoldings();
      }
    }
  }, 30000); // 30 seconds
}

function stopMarketDataRefresh() {
  if (marketDataRefreshInterval) {
    clearInterval(marketDataRefreshInterval);
    marketDataRefreshInterval = null;
  }
}

// MFA Functions
async function verifyMfa(event) {
  event.preventDefault();

  // Prevent duplicate submissions
  if (isMfaSubmitting) {
    console.log('MFA verification already in progress, ignoring duplicate submission');
    return;
  }

  if (!pendingMfaChallenge) {
    showMessage('mfa-message', 'No active MFA challenge. Please login again.', true);
    clearMfaState();
    showAuthSection();
    return;
  }

  const code = document.getElementById('mfa-code').value;

  if (!code || code.length !== 6) {
    showMessage('mfa-message', 'Please enter a valid 6-digit code', true);
    return;
  }

  isMfaSubmitting = true;

  try {
    // Both registration and login verification use the same endpoint
    const response = await makeApiCall('/v1/auth/verify-login', {
      method: 'POST',
      body: JSON.stringify({
        challenge_id: pendingMfaChallenge,
        code: code
      })
    });

    if (response.ok) {
      const data = await response.json();
      
      // Save token and login
      currentToken = data.token;
      localStorage.setItem('brokerx_token', currentToken);
      sessionStorage.removeItem('pending_user_id');
      
      showMessage('auth-message', 'Verification successful! You are now logged in.');
      
      clearMfaState();
      await loadUserProfile();
      showDashboard();
    } else {
      const error = await response.json();
      showMessage('mfa-message', error.message || 'Verification failed', true);
      isMfaSubmitting = false; // Reset flag on error
    }
  } catch (err) {
    showMessage('mfa-message', 'Network error occurred', true);
    console.error('MFA verification error:', err);
    isMfaSubmitting = false; // Reset flag on error
  }
}

async function resendMfaCode() {
  if (!pendingMfaChallenge) {
    showMessage('mfa-message', 'No active MFA challenge', true);
    return;
  }

  // Simulate resending MFA code
  showMessage('mfa-message', 'Code resent! Demo code: 123456');
}

function showMfaVerification(title, description) {
  // Hide other sections
  document.getElementById('auth-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.add('hidden');
  document.getElementById('profile-section').classList.add('hidden');
  document.getElementById('trading-section').classList.add('hidden');
  document.getElementById('orders-section').classList.add('hidden');
  
  // Show MFA section
  document.getElementById('mfa-section').classList.remove('hidden');
  document.getElementById('mfa-title').textContent = title;
  document.getElementById('mfa-description').textContent = description;
  document.getElementById('mfa-code').value = '';
  document.getElementById('mfa-code').focus();
}

function clearMfaState() {
  pendingMfaChallenge = null;
  mfaStep = null;
  isMfaSubmitting = false; // Reset submission flag
  document.getElementById('mfa-section').classList.add('hidden');
}

function cancelMfa() {
  clearMfaState();
  showAuthSection();
}

// UI Navigation Functions
function showAuthSection() {
  document.getElementById('auth-section').classList.remove('hidden');
  document.getElementById('mfa-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.add('hidden');
  document.getElementById('profile-section').classList.add('hidden');
  document.getElementById('market-data-section').classList.add('hidden');
  document.getElementById('trading-section').classList.add('hidden');
  document.getElementById('orders-section').classList.add('hidden');
  stopMarketDataRefresh();
  clearMfaState();
}

function showDashboard() {
  document.getElementById('auth-section').classList.add('hidden');
  document.getElementById('mfa-section').classList.add('hidden');
  document.getElementById('dashboard-section').classList.remove('hidden');
  document.getElementById('profile-section').classList.remove('hidden');
  document.getElementById('market-data-section').classList.remove('hidden');
  document.getElementById('holdings-section').classList.remove('hidden');
  document.getElementById('trading-section').classList.remove('hidden');
  document.getElementById('orders-section').classList.remove('hidden');

  // Load data when showing dashboard
  // Load market data first, then holdings (so prices are available)
  loadMarketData().then(() => {
    loadHoldings();
  });
  loadOrders();
  startMarketDataRefresh();
}

function showLogin() {
  document.getElementById('login-form').classList.remove('hidden');
  document.getElementById('register-form').classList.add('hidden');
}

function showRegister() {
  document.getElementById('login-form').classList.add('hidden');
  document.getElementById('register-form').classList.remove('hidden');
}

// Health check function (for development)
async function checkApiHealth() {
  try {
    const response = await fetch(`${API_BASE_URL}/v1/health`);
    if (response.ok) {
      const data = await response.json();
      console.log('API Health:', data);
      return true;
    }
  } catch (err) {
    console.error('API Health check failed:', err);
  }
  return false;
}

// Check API health on load
checkApiHealth();

// Auto-submit MFA code when 6 digits are entered
function setupMfaAutoSubmit() {
  const mfaCodeInput = document.getElementById('mfa-code');
  if (mfaCodeInput) {
    mfaCodeInput.addEventListener('input', function(e) {
      // Only allow numeric input
      e.target.value = e.target.value.replace(/[^0-9]/g, '');
      
      // Auto-submit when 6 digits are entered
      if (e.target.value.length === 6) {
        // Small delay to allow user to see the complete code
        setTimeout(() => {
          const form = e.target.closest('form');
          if (form) {
            form.dispatchEvent(new Event('submit'));
          }
        }, 500);
      }
    });

    // Focus the input when MFA section becomes visible
    const observer = new MutationObserver(function(mutations) {
      mutations.forEach(function(mutation) {
        if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
          const mfaSection = document.getElementById('mfa-section');
          if (mfaSection && !mfaSection.classList.contains('hidden')) {
            setTimeout(() => mfaCodeInput.focus(), 100);
          }
        }
      });
    });

    const mfaSection = document.getElementById('mfa-section');
    if (mfaSection) {
      observer.observe(mfaSection, { attributes: true });
    }
  }
}
