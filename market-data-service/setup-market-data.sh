#!/usr/bin/env bash

# Market Data Service Quick Setup Script
# This script initializes the market data service with sample data

echo "🚀 Market Data Service Setup"
echo ""

# Load environment variables
if [ -f ../.env.microservices ]; then
    export $(cat ../.env.microservices | grep -v '^#' | xargs)
elif [ -f ../.env ]; then
    export $(cat ../.env | grep -v '^#' | xargs)
else
    echo "⚠️  Warning: No .env file found. Using default port 3003."
fi

MARKET_DATA_PORT=${MARKET_DATA_SERVICE_PORT:-3003}

# Check if service is running
echo "📡 Checking if Market Data Service is running..."
if curl -s http://localhost:${MARKET_DATA_PORT}/health > /dev/null; then
    echo "✅ Service is running"
else
    echo "❌ Service is not running. Please start it first with:"
    echo "   ./start-microservices.sh"
    exit 1
fi

echo ""
echo "📊 Initializing market data with default symbols..."
response=$(curl -s -X POST http://localhost:${MARKET_DATA_PORT}/api/v1/initialize)
echo $response | jq '.'

echo ""
echo "📈 Running initial price simulation..."
response=$(curl -s -X POST http://localhost:${MARKET_DATA_PORT}/api/v1/simulate)
echo $response | jq '.'

echo ""
echo "📋 Available symbols:"
curl -s http://localhost:${MARKET_DATA_PORT}/api/v1/symbols | jq '.symbols'

echo ""
echo "💰 Sample current prices:"
curl -s "http://localhost:${MARKET_DATA_PORT}/api/v1/prices?symbols=AAPL,GOOGL,MSFT" | jq '.prices[] | {symbol, price}'

echo ""
echo "✅ Market Data Service is ready!"
echo ""
echo "📚 API Documentation: http://localhost:${MARKET_DATA_PORT}/swagger-ui"
echo "🔗 Service URL: http://localhost:${MARKET_DATA_PORT}"
