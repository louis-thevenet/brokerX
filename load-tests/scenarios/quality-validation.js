import { sleep } from 'k6';
import { ApiClient } from '../lib/api-client.js';
import { getRandomSymbol, thinkTime } from '../lib/utils.js';
import { config, scenarios } from '../config.js';

// Test configuration
export const options = {
  scenarios: {
    quality_validation_high_throughput: {
      executor: 'constant-vus',
      vus: 100, // Constant 100 users placing orders
      duration: '5m', // 5 minutes of sustained load
    },
  },
  thresholds: {
    ...config.thresholds,
    // Override for high throughput test: target 800 orders/s
    'http_reqs{name:create_order}': ['rate>800'],
  },
  setupTimeout: '5m', // Increased for creating more users
};

/**
 * Setup phase: Create test users with billions in funds
 * Creates 100 users for high-throughput testing to avoid contention
 */
export function setup() {
  console.log('Setting up test users for high-throughput quality validation (800 BUY orders/s target)...');
  const client = new ApiClient();
  const testUsers = [];

  // Create 100 test users (enough for constant-vus scenario)
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
        client.setUserId(authResult.user_id || userId);

        // Deposit billions for testing - unlimited buying power
        const depositSuccess = client.deposit(10000000000); // $10 billion for unlimited BUY orders

        if (depositSuccess) {
          testUsers.push({
            ...userData,
            userId,
          });
          console.log(`Created user ${i + 1}/${userCount}: ${userData.email} with $10B balance`);
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
  console.log(`Target: 800 BUY orders/second sustained with ONLY order creation requests`);
  console.log(`Each VU will authenticate once, then continuously place BUY orders`);
  return { users: testUsers };
}

// Per-VU state: each VU gets its own authenticated client
let vuClient = null;
let vuUser = null;

/**
 * Scenario: High-throughput quality validation - targeting 800 BUY orders/second
 * Each VU authenticates once, then continuously places BUY orders with no extra requests
 */
export default function(data) {
  if (!data || !data.users || data.users.length === 0) {
    console.error('No test users available');
    return;
  }

  // First iteration for this VU: authenticate once
  if (!vuClient || !vuUser) {
    vuClient = new ApiClient();

    // Assign a unique user to this VU (round-robin distribution)
    const vuIndex = __VU - 1; // VU IDs are 1-based
    const userIndex = vuIndex % data.users.length;
    vuUser = data.users[userIndex];

    // Authenticate once for this VU
    const authResult = vuClient.authenticate(vuUser.email, vuUser.password);
    if (!authResult) {
      console.error(`VU ${__VU}: Authentication failed for ${vuUser.email}`);
      return;
    }

    console.log(`VU ${__VU}: Authenticated as ${vuUser.email}`);
  }

  // Now just place orders - no login, no user info fetch
  // Only BUY orders (users have billions, no need to sell)
  const orderType = 'buy';
  const symbol = getRandomSymbol();
  const quantity = Math.floor(Math.random() * 3) + 1; // 1-3 shares

  // Place the order - this is the ONLY HTTP request in the hot path
  const orderResult = vuClient.createOrder(symbol, 'market', orderType, quantity);

  if (orderResult) {
    // Minimal logging for high throughput
    if (Math.random() < 0.001) { // Log only 0.1% of orders to reduce overhead
      console.log(`✓ BUY ${quantity} ${symbol} @ market`);
    }
  } else {
    // Still log errors
    if (Math.random() < 0.1) { // Log 10% of errors
      console.error(`✗ Order failed: BUY ${quantity} ${symbol}`);
    }
  }

  // Minimal think time for maximum throughput
  // With 100 VUs and 0.01-0.05s sleep, we get: 100 VUs / ~0.03s avg = ~3333 iterations/s
  // But each VU can only do so many requests due to network/processing time
  // Expect around 800-1200 orders/s with this configuration
  sleep(thinkTime(0.01, 0.05));
}

/**
 * Teardown phase: Optional cleanup
 */
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('High-Throughput Quality Validation Test Completed');
  console.log('='.repeat(80));
  if (data && data.users) {
    console.log(`Tested with ${data.users.length} users (each VU had dedicated user)`);
    console.log(`Target throughput: 800 BUY orders/second`);
    console.log(`Strategy: Constant VUs, each authenticated ONCE, then only placing BUY orders`);
    console.log(`No /login or /user requests during test - ONLY order creation`);
    console.log('Check the summary for actual throughput achieved');
  }
  console.log('='.repeat(80));
}
