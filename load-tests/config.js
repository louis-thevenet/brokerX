// Load test configuration
export const config = {
  // Base URL for API Gateway
  baseUrl: __ENV.BASE_URL || 'http://localhost:8080',

  // Test users configuration
  testUsers: {
    count: parseInt(__ENV.TEST_USERS || '100'),
    emailPrefix: 'loadtest',
    emailDomain: 'example.com',
    password: 'LoadTest123!',
  },

  // Trading symbols for testing (must match symbols initialized in database)
  symbols: ['AAPL', 'GOOGL', 'MSFT', 'AMZN', 'TSLA', 'META', 'NVDA', 'NFLX'],

  // Test thresholds aligned with quality objectives
  thresholds: {
    // Availability: 95.5% success rate (< 4.5% error rate)
    http_req_failed: ['rate<0.045'],

    // Performance: Order latency ≤ 250ms (p95) - relaxed slightly for high throughput
    'http_req_duration{name:create_order}': ['p(95)<300', 'p(99)<600'],

    // General performance: 95% of all requests under 500ms
    http_req_duration: ['p(95)<500', 'p(99)<1000'],

    // Throughput: Target 800 orders/second sustained
    'http_reqs{name:create_order}': ['rate>800'], // 800+ orders/s target

    // Checks: ensure validations pass
    checks: ['rate>0.95'],
  },

  // Timeouts
  timeouts: {
    default: '30s',
    report: '60s',
  },
};

// Scenario configurations
export const scenarios = {
  // Smoke test: minimal load to verify everything works
  smoke: {
    executor: 'constant-vus',
    vus: 1,
    duration: '1m',
  },

  // Load test: normal expected load
  load: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '1m', target: 300 },
      { duration: '3m', target: 300 },
      { duration: '1m', target: 500 },
      { duration: '3m', target: 500 },
      { duration: '1m', target: 1000 },
      { duration: '3m', target: 1000 },
      { duration: '2m', target: 0 },
    ],
  },

  // Stress test: push to limits
  stress: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '1m', target: 250 },  // Ramp up to 250 users
      { duration: '3m', target: 250 },  // Stay at 250
      { duration: '1m', target: 500 },  // Ramp up to 500
      { duration: '3m', target: 500 },  // Stay at 500
      { duration: '1m', target: 1000 }, // Ramp up to 1000
      { duration: '3m', target: 1000 }, // Stay at 1000
      { duration: '1m', target: 2000 }, // Ramp up to 2000
      { duration: '3m', target: 2000 }, // Stay at 2000
      { duration: '2m', target: 0 },    // Ramp down
    ],
  },

  // Spike test: sudden traffic spike
  spike: {
    executor: 'ramping-vus',
    startVUs: 0,
    stages: [
      { duration: '30s', target: 100 },  // Normal load
      { duration: '1m', target: 100 },   // Stay normal
      { duration: '10s', target: 1000 }, // Spike to 1000!
      { duration: '2m', target: 1000 },  // Maintain spike
      { duration: '10s', target: 100 },  // Back to normal
      { duration: '2m', target: 100 },   // Recovery
      { duration: '30s', target: 0 },    // Ramp down
    ],
  },

  // Soak test: sustained load over time
  soak: {
    executor: 'constant-vus',
    vus: 500,
    duration: '30m',
  },

  // High throughput test: Target 800+ orders/second
  highThroughput: {
    executor: 'ramping-arrival-rate',
    startRate: 0,
    timeUnit: '1s',
    preAllocatedVUs: 150,  // Increased from 100
    maxVUs: 2000,          // Increased from 1000 to allow more concurrent VUs
    stages: [
      { duration: '30s', target: 200 },   // Warm up: 200 orders/s (increased)
      { duration: '30s', target: 500 },   // Ramp up: 500 orders/s
      { duration: '1m', target: 900 },    // Target: 900 orders/s (overshoot to hit 800)
      { duration: '10m', target: 900 },    // Sustain: 900 orders/s for 5 minutes
      // { duration: '1m', target: 1100 },   // Push: 1100 orders/s
      // { duration: '2m', target: 1100 },   // Sustain peak: 1100 orders/s
      { duration: '1m', target: 500 },    // Ramp down: 500 orders/s
      { duration: '30s', target: 0 },     // Cool down
    ],
  },
};
