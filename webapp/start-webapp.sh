#!/usr/bin/env bash

# Start BrokerX Web Application
# Serves the static HTML/JS files on http://localhost:3000

set -e

echo "🌐 Starting BrokerX Web Application..."

# Check if API Gateway is running
API_HEALTH=$(curl -s http://localhost:8080/health 2>/dev/null || echo "failed")
if [[ $API_HEALTH == "failed" ]]; then
    echo "⚠️  Warning: API Gateway doesn't seem to be running on http://localhost:8080"
    echo "   Make sure to start the microservices first:"
    echo "   ./start-microservices.sh"
    echo ""
fi

# Try different methods to serve the files
if command -v python3 &> /dev/null; then
    echo "📡 Starting HTTP server with Python on port 3000..."
    echo "🔗 Open your browser to: http://localhost:3000"
    echo ""
    python3 -m http.server 3000
elif command -v python &> /dev/null; then
    echo "📡 Starting HTTP server with Python 2 on port 3000..."
    echo "🔗 Open your browser to: http://localhost:3000"
    echo ""
    python -m SimpleHTTPServer 3000
elif command -v npx &> /dev/null; then
    echo "📡 Starting HTTP server with Node.js on port 3000..."
    echo "🔗 Open your browser to: http://localhost:3000"
    echo ""
    npx http-server -p 3000
else
    echo "❌ No suitable HTTP server found."
    echo "   Please install Python 3 or Node.js to serve the web app."
    echo "   Alternatively, serve the files with any web server."
    exit 1
fi
