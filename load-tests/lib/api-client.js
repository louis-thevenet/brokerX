import http from 'k6/http';
import { check } from 'k6';
import { config } from '../config.js';

const BASE_URL = config.baseUrl;

/**
 * API Client for BrokerX load testing
 */
export class ApiClient {
  constructor() {
    this.baseUrl = BASE_URL;
    this.token = null;
    this.userId = null;
  }

  /**
   * Set authentication token
   */
  setToken(token) {
    this.token = token;
  }

  /**
   * Set user ID
   */
  setUserId(userId) {
    this.userId = userId;
  }

  /**
   * Get headers with optional authentication
   */
  getHeaders(authenticated = false) {
    const headers = {
      'Content-Type': 'application/json',
    };
    
    if (authenticated && this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }
    
    return headers;
  }

  // ========== User Service APIs ==========

  /**
   * Register a new user
   */
  registerUser(email, password, firstname, surname, initialBalance = 10000.0) {
    const payload = JSON.stringify({
      email,
      password,
      firstname,
      surname,
      initial_balance: initialBalance,
    });

    const response = http.post(
      `${this.baseUrl}/v1/auth/register`,
      payload,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'register user: status 201': (r) => r.status === 201,
    });

    if (response.status === 201) {
      const data = response.json();
      return data.user_id || data.id;
    }
    
    return null;
  }

  /**
   * Authenticate user and get token (with MFA flow)
   */
  authenticate(email, password) {
    // Step 1: Login and get challenge_id
    const loginPayload = JSON.stringify({ email, password });

    const loginResponse = http.post(
      `${this.baseUrl}/v1/auth/login`,
      loginPayload,
      { headers: this.getHeaders(false) }
    );

    check(loginResponse, {
      'authenticate: login status 200': (r) => r.status === 200,
      'authenticate: has challenge_id': (r) => r.json('challenge_id') !== undefined,
    });

    if (loginResponse.status !== 200) {
      return null;
    }

    const loginData = loginResponse.json();
    const challengeId = loginData.challenge_id;
    const userId = loginData.user_id;

    if (!challengeId) {
      return null;
    }

    // Step 2: Verify MFA with dummy code (MFA bypass accepts any code)
    const verifyPayload = JSON.stringify({
      challenge_id: challengeId,
      code: '000000', // Any code works with MFA_BYPASS=true
    });

    const verifyResponse = http.post(
      `${this.baseUrl}/v1/auth/verify-login`,
      verifyPayload,
      { headers: this.getHeaders(false) }
    );

    check(verifyResponse, {
      'authenticate: verify status 200': (r) => r.status === 200,
      'authenticate: has token': (r) => r.json('token') !== undefined,
    });

    if (verifyResponse.status === 200) {
      const data = verifyResponse.json();
      if (data.token) {
        this.setToken(data.token);
      }
      if (data.user_id || userId) {
        this.setUserId(data.user_id || userId);
      }
      return data;
    }
    
    return null;
  }

  /**
   * Get user details (authenticated user profile)
   */
  getUser() {
    const response = http.get(
      `${this.baseUrl}/v1/me`,
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'get user: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get user portfolio (holdings are included in /v1/me response)
   */
  getPortfolio() {
    // Portfolio is included in the user profile
    return this.getUser();
  }

  /**
   * Deposit funds
   */
  deposit(amount) {
    const payload = JSON.stringify({ amount });

    const response = http.post(
      `${this.baseUrl}/v1/me/deposit`,
      payload,
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'deposit: status 200': (r) => r.status === 200,
    });

    return response.status === 200;
  }

  // ========== Market Data Service APIs ==========

  /**
   * Get current price for a symbol
   */
  getCurrentPrice(symbol) {
    const response = http.get(
      `${this.baseUrl}/v1/market/prices/${symbol}`,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'get current price: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get current prices for multiple symbols
   */
  getCurrentPrices(symbols) {
    const symbolsParam = symbols.join(',');
    const response = http.get(
      `${this.baseUrl}/v1/market/prices?symbols=${symbolsParam}`,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'get current prices: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get all current prices
   */
  getAllCurrentPrices() {
    const response = http.get(
      `${this.baseUrl}/v1/market/prices`,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'get all prices: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get historical prices
   */
  getHistoricalPrices(symbol, limit = 100) {
    const response = http.get(
      `${this.baseUrl}/v1/market/prices/${symbol}/history?limit=${limit}`,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'get historical prices: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get all available symbols
   */
  getSymbols() {
    const response = http.get(
      `${this.baseUrl}/v1/market/symbols`,
      { headers: this.getHeaders(false) }
    );

    check(response, {
      'get symbols: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  // ========== Order Service APIs ==========

  /**
   * Create a new order
   */
  createOrder(symbol, orderType, orderSide, quantity, price = null) {
    if (!this.userId) {
      console.error('Cannot create order: no userId available');
      return null;
    }

    // Rust expects:
    // Market: { order_type: { type: "market" }, ... }
    // Limit: { order_type: { type: "limit", price: 123.45 }, price: 123.45, ... }
    let order_type;
    let payload = {
      client_id: this.userId,
      symbol,
      order_side: orderSide,
      quantity,
    };

    if (orderType === 'market') {
      order_type = { type: 'market' };
      payload.order_type = order_type;
      // Do not include price
    } else if (orderType === 'limit') {
      if (price === null || price === undefined) {
        console.error('Price is required for limit orders');
        return null;
      }
      order_type = { type: 'limit', price: price };
      payload.order_type = order_type;
      payload.price = price;
    } else {
      console.error(`Unknown order type: ${orderType}`);
      return null;
    }

    // Debug: print payload
    // console.log('Order payload:', JSON.stringify(payload));

    const response = http.post(
      `${this.baseUrl}/v1/orders`,
      JSON.stringify(payload),
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'create order: status 201': (r) => r.status === 201,
    });

    if (response.status !== 201) {
      console.error(`Failed to create order. Status: ${response.status}, Body: ${response.body}`);
    }

    return response.status === 201 ? response.json() : null;
  }

  /**
   * Get order by ID
   */
  getOrder(orderId) {
    const response = http.get(
      `${this.baseUrl}/v1/orders/${orderId}`,
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'get order: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Get user orders (paginated)
   */
  getUserOrders(page = 1, limit = 50) {
    if (!this.userId) {
      console.error('Cannot get orders: no userId available');
      return null;
    }

    const response = http.get(
      `${this.baseUrl}/v1/orders?client_id=${this.userId}&page=${page}&limit=${limit}`,
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'get user orders: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Update an order
   */
  updateOrder(orderId, quantity = null, price = null) {
    const payload = {};
    if (quantity !== null) payload.quantity = quantity;
    if (price !== null) payload.price = price;

    const response = http.put(
      `${this.baseUrl}/v1/orders/${orderId}`,
      JSON.stringify(payload),
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'update order: status 200': (r) => r.status === 200,
    });

    return response.status === 200 ? response.json() : null;
  }

  /**
   * Cancel an order
   */
  cancelOrder(orderId) {
    const response = http.del(
      `${this.baseUrl}/v1/orders/${orderId}`,
      null,
      { headers: this.getHeaders(true) }
    );

    check(response, {
      'cancel order: status 200 or 204': (r) => 
        r.status === 200 || r.status === 204,
    });

    return response.status === 200 || response.status === 204;
  }

  // ========== Health Checks ==========

  /**
   * Check API Gateway health
   */
  healthCheck() {
    const response = http.get(`${this.baseUrl}/v1/health`);
    
    check(response, {
      'health check: status 200': (r) => r.status === 200,
    });

    return response.status === 200;
  }
}
