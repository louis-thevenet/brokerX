import { sleep } from 'k6';
import { SharedArray } from 'k6/data';
import { ApiClient } from '../lib/api-client.js';
import {
  generateOrderData,
  getRandomSymbol,
  thinkTime
} from '../lib/utils.js';
import { config, scenarios } from '../config.js';

// Test configuration
export const options = {
  scenarios: {
    user_journey: scenarios.load,
  },
  thresholds: config.thresholds,
  setupTimeout: '2m',
};

/**
 * Setup phase: Create test users
 */
export function setup() {
  console.log('Setting up test users for complete journey...');
  const client = new ApiClient();
  const testUsers = [];

  // Create 10 test users
  for (let i = 0; i < 10; i++) {
    const userData = {
      email: `journey_user_${i}_${Date.now()}@example.com`,
      password: config.testUsers.password,
      firstname: 'Journey',
      surname: `User${i}`,
      initialBalance: 10000,
    };

    const userId = client.registerUser(
      userData.email,
      userData.password,
      userData.firstname,
      userData.surname,
      userData.initialBalance
    );

    if (userId) {
      testUsers.push({
        ...userData,
        userId,
      });
    }

    sleep(0.5);
  }

  console.log(`Created ${testUsers.length} test users`);
  return { users: testUsers };
}

/**
 * Scenario: Complete user journey
 * Simulates realistic user behavior from login to trading
 */
export default function(data) {
  if (!data || !data.users || data.users.length === 0) {
    console.error('No test users available');
    return;
  }

  const client = new ApiClient();

  // Pick a random user
  const user = data.users[Math.floor(Math.random() * data.users.length)];

  // Step 1: Authenticate
  const authResult = client.authenticate(user.email, user.password);
  if (!authResult) {
    console.error('Authentication failed');
    return;
  }
  sleep(thinkTime(1, 2));

  // Step 1b: Deposit funds to ensure balance
  client.deposit(10000);
  sleep(0.5);

  // Step 2: View user profile
  client.getUser();
  sleep(thinkTime(1, 3));

  // Step 3: Check portfolio
  client.getPortfolio();
  sleep(thinkTime(2, 4));

  // Step 4: Browse market data
  client.getAllCurrentPrices();
  sleep(thinkTime(2, 5));

  // Step 5: Check specific stocks
  const symbol1 = getRandomSymbol();
  client.getCurrentPrice(symbol1);
  sleep(thinkTime(1, 2));

  client.getHistoricalPrices(symbol1, 30);
  sleep(thinkTime(2, 4));

  // Step 6: Place a market buy order (small quantity to avoid concentration limit)
  const buyOrderData = generateOrderData();
  buyOrderData.orderType = 'market';
  buyOrderData.orderSide = 'buy';
  buyOrderData.quantity = 2; // Reduced from 5 to avoid concentration limit

  const buyOrder = client.createOrder(
    buyOrderData.symbol,
    buyOrderData.orderType,
    buyOrderData.orderSide,
    buyOrderData.quantity
  );
  sleep(thinkTime(1, 2));

  // Step 7: Check order status
  if (buyOrder && buyOrder.id) {
    client.getOrder(buyOrder.id);
  }
  sleep(thinkTime(1, 3));

  // Step 8: View updated portfolio
  client.getPortfolio();
  sleep(thinkTime(2, 3));

  // Step 9: Check another stock
  const symbol2 = getRandomSymbol();
  client.getCurrentPrice(symbol2);
  sleep(thinkTime(1, 2));

  // Step 10: Place a limit sell order for the symbol just bought, only if buy succeeded
  if (buyOrder && buyOrderData.symbol) {
    const sellSymbol = buyOrderData.symbol;
    const priceForSell = client.getCurrentPrice(sellSymbol);

    if (priceForSell && typeof priceForSell.price === 'number' && priceForSell.price > 0) {
      // Round to nearest cent to conform to tick size
      const limitPrice = Math.round(priceForSell.price * 1.05 * 100) / 100;
      const sellOrder = client.createOrder(
        sellSymbol,
        'limit',
        'sell',
        2,
        limitPrice
      );
      sleep(thinkTime(1, 2));


    } else {
      console.log(`Failed to get valid price for ${sellSymbol}:`, priceForSell);
      console.log('Skipping limit sell order.');
    }
  } else {
    console.log('Skipping limit sell order because market buy failed.');
  }

  // Step 12: View all user orders
  client.getUserOrders();
  sleep(thinkTime(2, 3));

  // Step 13: Final portfolio check
  client.getPortfolio();
  sleep(thinkTime(1, 2));
}

/**
 * Teardown phase
 */
export function teardown(data) {
  console.log('Complete user journey test finished');
}
