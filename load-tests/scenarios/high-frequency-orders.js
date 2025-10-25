import { sleep } from 'k6';
import { SharedArray } from 'k6/data';
import { ApiClient } from '../lib/api-client.js';
import {
  generateOrderData,
  getRandomSymbol,
  thinkTime
} from '../lib/utils.js';
import { config, scenarios } from '../config.js';

// Shared array of test users (created in setup)
const users = new SharedArray('users', function() {
  // This will be populated during setup phase
  return [];
});

// Test configuration
export const options = {
  scenarios: {
    high_frequency_trading: scenarios.load,
  },
  thresholds: config.thresholds,
  setupTimeout: '2m',
};

/**
 * Setup phase: Create test users
 */
export function setup() {
  console.log('Setting up test users...');
  const client = new ApiClient();
  const testUsers = [];

  // Create 20 test users for high-frequency trading
  for (let i = 0; i < 20; i++) {
    const userData = {
      email: `hft_trader_${i}_${Date.now()}@example.com`,
      password: config.testUsers.password,
      firstname: 'HFT',
      surname: `Trader${i}`,
    };

    const userId = client.registerUser(
      userData.email,
      userData.password,
      userData.firstname,
      userData.surname
    );

    if (userId) {
      // Authenticate and get token
      const authResult = client.authenticate(userData.email, userData.password);
      if (authResult && authResult.token) {
        // Token is already set by authenticate(), just need to ensure userId is set
        client.setUserId(authResult.user_id || userId);
        
        // Deposit initial funds
        const depositSuccess = client.deposit(5000000); // $5M for high-frequency trading
        
        if (depositSuccess) {
          testUsers.push({
            ...userData,
            userId,
          });
        }
      }
    }

    sleep(0.5); // Avoid overwhelming the registration endpoint
  }

  console.log(`Created ${testUsers.length} test users with initial deposits`);
  return { users: testUsers };
}

/**
 * Scenario: High-frequency order placement
 * Simulates aggressive trading with rapid order creation 
 */
export default function(data) {
  if (!data || !data.users || data.users.length === 0) {
    console.error('No test users available');
    return;
  }

  const client = new ApiClient();

  // Pick a random user
  const user = data.users[Math.floor(Math.random() * data.users.length)];

  // Authenticate
  const authResult = client.authenticate(user.email, user.password);
  if (!authResult) {
    console.error('Authentication failed');
    return;
  }

  // Deposit funds to ensure balance
  client.deposit(5000000);
  sleep(0.5);

  // Rapid trading cycle
  let lastSymbol = null;
  for (let i = 0; i < 10; i++) {
    // Get current market price
    const symbol = getRandomSymbol();
    // Ensure we don't repeat the same symbol in immediate succession
    if (symbol === lastSymbol) {
      sleep(0.5); // Wait a bit longer before retrying a new symbol
      continue;
    }
    lastSymbol = symbol;
    const priceData = client.getCurrentPrice(symbol);

    if (!priceData || !priceData.price) {
      console.log(`No price data available for ${symbol}, skipping`);
      continue;
    }

    // Place a market buy order
    const buyOrder = client.createOrder(
      symbol,
      'market',
      'buy',
      2,
    );
    // console.log(`Buy order response for ${symbol}:`, JSON.stringify(buyOrder));
    let buyStatus = buyOrder && buyOrder.status && buyOrder.status.status ? buyOrder.status.status.toLowerCase() : '';
    if (!buyOrder || buyOrder.error) {
      // console.warn(`Buy order failed for ${symbol}: ${buyOrder && buyOrder.message ? buyOrder.message : 'Unknown error'}`);
      continue;
    } else if (buyStatus === 'rejected') {
      // console.warn(`Buy order rejected for ${symbol}: ${buyOrder.reason || JSON.stringify(buyOrder)}`);
      continue;
    } else if (buyStatus !== 'filled' && buyStatus !== 'partially_filled') {
      console.warn(`Buy order for ${symbol} not filled (status: ${buyStatus}), skipping sell.`);
      continue;
    }

    // Wait for holdings to update after buy order
    let maxRetries = 20; // Increased retries for backend consistency
    let retry = 0;
    let sharesOwned = 0;
    while (retry < maxRetries) {
      const portfolio = client.getPortfolio ? client.getPortfolio() : null;
      if (portfolio && typeof portfolio === 'object') {
        if (portfolio.holdings && portfolio.holdings[symbol] && portfolio.holdings[symbol].quantity) {
          sharesOwned = portfolio.holdings[symbol].quantity;
        } else if (Array.isArray(portfolio)) {
          const holding = portfolio.find(item => item.symbol === symbol);
          if (holding && holding.quantity) {
            sharesOwned = holding.quantity;
          }
        }
      }
      if (sharesOwned >= 2) break;
      sleep(0.3); // Slightly longer wait for backend update
      retry++;
    }

    if (sharesOwned >= 2) {
      const sellOrder = client.createOrder(
        symbol,
        'market',
        'sell',
        2,
      );
      if (!sellOrder || sellOrder.error) {
        // console.warn(`Sell order failed for ${symbol}: ${sellOrder && sellOrder.message ? sellOrder.message : 'Unknown error'}`);
      } else if (sellOrder.status && sellOrder.status === 'rejected') {
        // console.warn(`Sell order rejected for ${symbol}: ${sellOrder.reason || JSON.stringify(sellOrder)}`);
      }
    } else {
      console.log(`Not enough shares to sell ${symbol} after waiting`);
    }

    sleep(0.1); // Minimal pause before next iteration
  }

  // Check user's order history
  client.getUserOrders();

  sleep(1);
}

/**
 * Teardown phase: Optional cleanup
 */
export function teardown(data) {
  console.log('Test completed');
}
