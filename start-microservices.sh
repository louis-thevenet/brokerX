#!/usr/bin/env bash

# Start microservicesecho ""
echo "✅ All services started!"
echo ""
echo "🌐 API Gateway (Main Entry Point):"
echo "   🔗 http://localhost:${API_GATEWAY_PORT:-8080}"
echo "   📚 API Documentation: http://localhost:${API_GATEWAY_PORT:-8080}/swagger-ui/"
echo ""
echo "🔧 Internal Microservices:"
echo "   👤 User Service: http://localhost:${USER_SERVICE_PORT:-3001}"
echo "   📈 Order Service: http://localhost:${ORDER_SERVICE_PORT:-3002}"
echo "   📊 Market Data Service: http://localhost:${MARKET_DATA_SERVICE_PORT:-3003}"
echo ""
echo "💡 Use the API Gateway for all client requests!"
echo "   The microservices are for internal communication only."
echo ""
echo "Press Ctrl+C to stop all services..."

# Wait for interrupt
trap 'echo "🛑 Stopping services..."; kill $USER_PID $ORDER_PID $MARKET_DATA_PID $GATEWAY_PID 2>/dev/null; exit 0' INT
waitcript starts the user service, order service, and API gateway

set -e

echo "🚀 Starting BrokerX Microservices..."

# Load environment variables from centralized .env file
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
else
    echo "⚠️  Warning: .env file not found. Using default values."
fi

# Build services
echo "📦 Building services..."
cargo build --release -p user-service -p order-service -p market-data-service -p api_gateway

# Start services in background
echo "🔧 Starting User Service on port ${USER_SERVICE_PORT:-3001}..."
cargo build --release -p user-service
cargo run --release -p user-service &
USER_PID=$!

echo "⚡ Starting Order Service on port ${ORDER_SERVICE_PORT:-3002}..."
cargo build --release -p order-service 
cargo run --release -p order-service &
ORDER_PID=$!

echo "📊 Starting Market Data Service on port ${MARKET_DATA_SERVICE_PORT:-3003}..."
cargo build --release -p market-data-service
cargo run --release -p market-data-service &
MARKET_DATA_PID=$!

# echo "🌐 Starting API Gateway on port ${API_GATEWAY_PORT:-8080}..."
# cargo build --release -p api_gateway
# cargo run --release -p api_gateway &
# GATEWAY_PID=$!


echo ""
echo "✅ All services started!"
echo ""
echo "🌐 API Gateway (Main Entry Point):"
echo "   🔗 http://localhost:${API_GATEWAY_PORT:-8080}"
echo "   📚 API Documentation: http://localhost:${API_GATEWAY_PORT:-8080}/swagger-ui/"
echo ""
echo "� Internal Microservices:"
echo "   👤 User Service: http://localhost:${USER_SERVICE_PORT:-3001}"
echo "   📈 Order Service: http://localhost:${ORDER_SERVICE_PORT:-3002}"
echo ""
echo "💡 Use the API Gateway for all client requests!"
echo "   The microservices are for internal communication only."
echo ""
echo "Press Ctrl+C to stop all services..."

# Wait for interrupt
trap 'echo "🛑 Stopping services..."; kill $USER_PID $ORDER_PID $GATEWAY_PID 2>/dev/null; exit 0' INT
wait
