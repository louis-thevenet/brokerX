import { sleep } from 'k6';
import { ApiClient } from '../lib/api-client.js';
import { getRandomSymbol, thinkTime } from '../lib/utils.js';
import { config, scenarios } from '../config.js';

// Test configuration
export const options = {
  scenarios: {
    quality_validation_high_throughput: scenarios.highThroughput,
  },
  thresholds: {
    ...config.thresholds,
    // Override for high throughput test: target 800 orders/s
    'http_reqs{name:create_order}': ['rate>800'],
  },
  setupTimeout: '5m', // Increased for creating more users
};

/**
 * Setup phase: Create test users with funds and holdings
 * Creates 50 users for high-throughput testing to avoid contention
 */
export function setup() {
  console.log('Setting up test users for high-throughput quality validation (800 orders/s target)...');
  const client = new ApiClient();
  const testUsers = [];

  // Create 50 test users with sufficient funds for high-volume testing
  const userCount = 50;
  for (let i = 0; i < userCount; i++) {
    const userData = {
      email: `quality_ht_user_${i}_${Date.now()}@example.com`,
      password: config.testUsers.password,
      firstname: 'QualityHT',
      surname: `User${i}`,
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
        
        // Deposit substantial funds for testing
        const depositSuccess = client.deposit(500000); // $500k for high-throughput testing
        
        if (depositSuccess) {
          testUsers.push({
            ...userData,
            userId,
          });
          console.log(`Created user ${i + 1}/${userCount}: ${userData.email} with $500k balance`);
        } else {
          console.error(`Failed to deposit funds for user ${i + 1}`);
        }
      } else {
        console.error(`Failed to authenticate user ${i + 1}`);
      }
    } else {
      console.error(`Failed to register user ${i + 1}`);
    }

    // Reduce sleep time for faster setup
    if (i < userCount - 1) {
      sleep(0.2);
    }
  }

  console.log(`Successfully created ${testUsers.length} test users for high-throughput validation`);
  console.log(`Target: 800 orders/second sustained`);
  return { users: testUsers };
}

/**
 * Scenario: High-throughput quality validation - targeting 800 orders/second
 * Places orders that should pass all validation checks with minimal delays
 */
export default function(data) {
  if (!data || !data.users || data.users.length === 0) {
    console.error('No test users available');
    return;
  }

  const client = new ApiClient();

  // Pick a random user (should have balance from setup)
  const user = data.users[Math.floor(Math.random() * data.users.length)];

  // Authenticate
  const authResult = client.authenticate(user.email, user.password);
  if (!authResult) {
    console.error(`Authentication failed for ${user.email}`);
    return;
  }

  // Get user info to verify balance
  const userInfo = client.getUser();
  if (!userInfo) {
    console.error('Failed to get user info');
    return;
  }

  // Decide whether to BUY or SELL based on what the user has
  const hasHoldings = userInfo.holdings && Object.keys(userInfo.holdings).length > 0;
  const hasBalance = userInfo.balance > 1000;
  
  let orderType;
  let symbol;
  let quantity;
  
  // Randomly choose to BUY or SELL, but only if we have the resources
  const wantsToBuy = Math.random() < 0.5;
  
  if (wantsToBuy && hasBalance) {
    // Place a BUY order
    orderType = 'buy';
    symbol = getRandomSymbol();
    
    // For MARKET orders, the system will fetch the current price internally
    // No need to fetch it here - reduces latency and load on market-data-service
    
    // Use a small fixed quantity for high throughput
    // With $500k balance and typical stock prices ($50-200), 5 shares is very conservative
    quantity = Math.floor(Math.random() * 3) + 1; // 1-3 shares for maximum throughput
    
  } else if (!wantsToBuy && hasHoldings) {
    // Place a SELL order for a random holding
    orderType = 'sell';
    const symbols = Object.keys(userInfo.holdings);
    symbol = symbols[Math.floor(Math.random() * symbols.length)];
    const holding = userInfo.holdings[symbol];
    
    if (!holding || typeof holding.quantity !== 'number' || holding.quantity < 1) {
      console.warn(`Invalid holding for ${symbol}:`, JSON.stringify(holding));
      return;
    }
    
    const holdingQuantity = holding.quantity;
    
    // Sell a small portion for high throughput (10-30%)
    const sellPercentage = 0.1 + Math.random() * 0.2; // Sell 10-30%
    quantity = Math.max(1, Math.floor(holdingQuantity * sellPercentage));
    
    // Safety check: never sell more than we have
    if (quantity > holdingQuantity) {
      quantity = holdingQuantity;
    }
    
  } else {
    // Can't buy (no balance) or can't sell (no holdings), skip this iteration
    return;
  }

  // Place the order
  const orderResult = client.createOrder(symbol, 'market', orderType, quantity);
  
  if (orderResult) {
    // Minimal logging for high throughput
    if (Math.random() < 0.01) { // Log only 1% of orders to reduce overhead
      console.log(`✓ ${orderType.toUpperCase()} ${quantity} ${symbol} @ market`);
    }
  } else {
    console.error(`✗ Order failed: ${orderType.toUpperCase()} ${quantity} ${symbol}`);
  }

  // Minimal think time for high throughput
  sleep(thinkTime(0.05, 0.2)); // Reduced from 0.1-0.3
}

/**
 * Teardown phase: Optional cleanup
 */
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('High-Throughput Quality Validation Test Completed');
  console.log('='.repeat(80));
  if (data && data.users) {
    console.log(`Tested with ${data.users.length} users`);
    console.log(`Target throughput: 800 orders/second`);
    console.log(`Peak throughput: 1000 orders/second`);
    console.log('Check the summary for actual throughput achieved');
  }
  console.log('='.repeat(80));
}
