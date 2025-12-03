import { sleep } from 'k6';
import { ApiClient } from '../lib/api-client.js';
import { getRandomSymbol } from '../lib/utils.js';
import { config } from '../config.js';

// Test configuration - optimized for 1200 orders/s
export const options = {
  scenarios: {
    order_creation_throughput: {
      executor: 'constant-vus',
      vus: 100, // Increased for higher throughput
      duration: '5m',
    },
  },
  thresholds: {
    ...config.thresholds,
    'http_reqs{name:create_order}': ['rate>1200'], // Target 1200 orders/s
  },
  setupTimeout: '5m',
};

/**
 * Setup phase: Create test users with funding for order creation
 */
export function setup() {
  console.log('Setting up test users for order creation throughput test (1200 orders/s target)...');
  const client = new ApiClient();
  const testUsers = [];

  const userCount = 100; // One user per VU for maximum isolation
  for (let i = 0; i < userCount; i++) {
    const userData = {
      email: `order_test_${i}_${Date.now()}@example.com`,
      password: config.testUsers.password,
      firstname: 'OrderTest',
      surname: `User${i}`,
    };

    const userId = client.registerUser(
      userData.email,
      userData.password,
      userData.firstname,
      userData.surname
    );

    if (userId) {
      const authResult = client.authenticate(userData.email, userData.password);
      if (authResult && authResult.token) {
        client.setUserId(authResult.user_id || userId);

        // Deposit sufficient funds for unlimited buy orders
        const depositSuccess = client.deposit(10000000000); // $10 billion

        if (depositSuccess) {
          testUsers.push({
            ...userData,
            userId,
          });
          console.log(`Created user ${i + 1}/${userCount}`);
        }
      }
    }

    if (i < userCount - 1) {
      sleep(0.1);
    }
  }

  console.log(`Setup complete: ${testUsers.length} users ready`);
  console.log(`Target: 1200 orders/second (order creation only)`);
  return { users: testUsers };
}

// Per-VU state
let vuClient = null;
let vuUser = null;

/**
 * Main test: Order creation throughput - targeting 1200 orders/second
 */
export default function(data) {
  if (!data || !data.users || data.users.length === 0) {
    console.error('No test users available');
    return;
  }

  // One-time authentication per VU
  if (!vuClient || !vuUser) {
    vuClient = new ApiClient();
    const vuIndex = __VU - 1;
    const userIndex = vuIndex % data.users.length;
    vuUser = data.users[userIndex];

    const authResult = vuClient.authenticate(vuUser.email, vuUser.password);
    if (!authResult) {
      console.error(`VU ${__VU}: Authentication failed`);
      return;
    }
  }

  // Order creation - ONLY request in the hot path
  const symbol = getRandomSymbol();
  const quantity = Math.floor(Math.random() * 5) + 1; // 1-5 shares

  vuClient.createOrder(symbol, 'market', 'buy', quantity);

  // Target 1200 orders/s: 100 VUs / 1200 orders/s = 0.083s per iteration
  sleep(0.083);
}

/**
 * Teardown phase
 */
export function teardown(data) {
  console.log('='.repeat(80));
  console.log('Order Creation Throughput Test Completed');
  console.log('='.repeat(80));
  if (data && data.users) {
    console.log(`Tested with ${data.users.length} users`);
    console.log(`Target: 1200 orders/second`);
    console.log(`Method: Constant VUs, authenticated once, continuous order creation`);
  }
  console.log('='.repeat(80));
}
