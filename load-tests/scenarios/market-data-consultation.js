import { sleep } from 'k6';
import { ApiClient } from '../lib/api-client.js';
import { 
  getRandomSymbol, 
  getRandomSymbols,
  thinkTime 
} from '../lib/utils.js';
import { config, scenarios } from '../config.js';

// Test configuration
export const options = {
  scenarios: {
    market_data_consultation: scenarios.load,
  },
  thresholds: config.thresholds,
};

/**
 * Scenario: Market data consultation
 * Simulates users browsing market data, checking prices and historical data
 */
export default function () {
  const client = new ApiClient();

  // Health check
  client.healthCheck();
  sleep(1);

  // Get single stock price
  const symbol = getRandomSymbol();
  client.getCurrentPrice(symbol);
  sleep(thinkTime(1, 3));

  // Get multiple stock prices
  const symbols = getRandomSymbols(5);
  client.getCurrentPrices(symbols);
  sleep(thinkTime(1, 2));

  // Get all current prices (dashboard view)
  client.getAllCurrentPrices();
  sleep(thinkTime(2, 4));

  // Get historical prices
  client.getHistoricalPrices(symbol, 50);
  sleep(thinkTime(2, 5));

  // Browse another symbol
  const anotherSymbol = getRandomSymbol();
  client.getCurrentPrice(anotherSymbol);
  sleep(thinkTime(1, 2));
  
  // Get historical data for another symbol
  client.getHistoricalPrices(anotherSymbol, 30);
  sleep(thinkTime());
}
