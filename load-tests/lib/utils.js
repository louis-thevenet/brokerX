import { config } from '../config.js';

/**
 * Get random integer between min and max (inclusive)
 */
function randomIntBetween(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

/**
 * Get random item from array
 */
function randomItem(array) {
  return array[Math.floor(Math.random() * array.length)];
}

/**
 * Generate test user email
 */
export function generateTestUserEmail(index) {
  const { emailPrefix, emailDomain } = config.testUsers;
  return `${emailPrefix}_${index}@${emailDomain}`;
}

/**
 * Generate random user data
 */
export function generateUserData(index) {
  const firstnames = ['John', 'Jane', 'Bob', 'Alice', 'Charlie', 'Diana', 'Eve', 'Frank'];
  const surnames = ['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis'];
  
  return {
    email: generateTestUserEmail(index),
    password: config.testUsers.password,
    firstname: randomItem(firstnames),
    surname: randomItem(surnames),
    initialBalance: randomIntBetween(5000, 20000),
  };
}

/**
 * Get random symbol
 */
export function getRandomSymbol() {
  return randomItem(config.symbols);
}

/**
 * Get random symbols (multiple)
 */
export function getRandomSymbols(count = 3) {
  const shuffled = [...config.symbols].sort(() => 0.5 - Math.random());
  return shuffled.slice(0, Math.min(count, config.symbols.length));
}

/**
 * Generate random order data
 */
export function generateOrderData() {
  const symbol = getRandomSymbol();
  const orderTypes = ['market', 'limit'];
  const orderSides = ['buy', 'sell'];
  
  const orderType = randomItem(orderTypes);
  const orderSide = randomItem(orderSides);
  const quantity = randomIntBetween(1, 100);
  
  // For limit orders, add a price
  const price = orderType === 'limit' ? randomIntBetween(100, 500) : null;
  
  return {
    symbol,
    orderType,
    orderSide,
    quantity,
    price,
  };
}

/**
 * Get random sleep duration with jitter
 */
export function sleepWithJitter(minSeconds, maxSeconds) {
  return randomIntBetween(minSeconds, maxSeconds);
}

/**
 * Think time - simulates user pause between actions
 * Returns the duration in seconds (to be used with sleep())
 */
export function thinkTime(min = 1, max = 5) {
  return sleepWithJitter(min, max);
}
