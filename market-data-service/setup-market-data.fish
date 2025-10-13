#!/usr/bin/env fish

# Market Data Service Quick Setup Script
# This script initializes the market data service with sample data

echo "🚀 Market Data Service Setup"
echo ""

# Load environment variables
if test -f ../.env.microservices
    set -gx (cat ../.env.microservices | grep -v '^#' | grep -v '^$' | string split '=')
else if test -f ../.env
    set -gx (cat ../.env | grep -v '^#' | grep -v '^$' | string split '=')
else
    echo "⚠️  Warning: No .env file found. Using default port 3003."
end

set MARKET_DATA_PORT (if set -q MARKET_DATA_SERVICE_PORT; echo $MARKET_DATA_SERVICE_PORT; else; echo "3003"; end)

# Check if service is running
echo "📡 Checking if Market Data Service is running..."
if curl -s http://localhost:$MARKET_DATA_PORT/health > /dev/null
    echo "✅ Service is running"
else
    echo "❌ Service is not running. Please start it first with:"
    echo "   ./start-microservices.sh"
    exit 1
end

echo ""
echo "📊 Initializing market data with default symbols..."
set response (curl -s -X POST http://localhost:$MARKET_DATA_PORT/api/v1/initialize)
echo $response | jq '.'

echo ""
echo "📈 Running initial price simulation..."
set response (curl -s -X POST http://localhost:$MARKET_DATA_PORT/api/v1/simulate)
echo $response | jq '.'

echo ""
echo "📋 Available symbols:"
curl -s http://localhost:$MARKET_DATA_PORT/api/v1/symbols | jq '.symbols'

echo ""
echo "💰 Sample current prices:"
curl -s "http://localhost:$MARKET_DATA_PORT/api/v1/prices?symbols=AAPL,GOOGL,MSFT" | jq '.prices[] | {symbol, price}'

echo ""
echo "✅ Market Data Service is ready!"
echo ""
echo "📚 API Documentation: http://localhost:$MARKET_DATA_PORT/swagger-ui"
echo "🔗 Service URL: http://localhost:$MARKET_DATA_PORT"
