#!/usr/bin/env fish

# Quick setup script for BrokerX load testing

echo "🚀 BrokerX Load Test Setup"
echo "=========================="
echo ""

# Check if k6 is installed
if command -v k6 > /dev/null
    echo "✅ k6 is already installed"
    k6 version
else
    echo "❌ k6 is not installed"
    echo ""
    echo "To install k6:"
    echo ""
    echo "macOS:"
    echo "  brew install k6"
    echo ""
    echo "Linux (Debian/Ubuntu):"
    echo "  curl -s https://dl.k6.io/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/k6-archive-keyring.gpg"
    echo "  echo 'deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main' | sudo tee /etc/apt/sources.list.d/k6.list"
    echo "  sudo apt-get update"
    echo "  sudo apt-get install k6"
    echo ""
    echo "Windows:"
    echo "  choco install k6"
    echo ""
    echo "Or visit: https://k6.io/docs/getting-started/installation/"
    exit 1
end

echo ""
echo "📁 Creating results directory..."
mkdir -p results
echo "✅ Results directory created"

echo ""
echo "🔍 Checking if services are running..."

# Check if API Gateway is responding
set -l api_url "http://localhost:8080/v1/health"

if curl -s -f -o /dev/null $api_url
    echo "✅ API Gateway is running at http://localhost:8080"
else
    echo "⚠️  API Gateway is not responding at http://localhost:8080"
    echo ""
    echo "Start your services with:"
    echo "  docker-compose up -d"
    echo ""
    echo "Or start them manually if not using Docker."
end

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Ensure all services are running (docker-compose up -d)"
echo "2. Run a quick smoke test:"
echo "   ./run-tests.fish market-data local"
echo ""
echo "Available test scenarios:"
echo "  - market-data         Quick market data browsing"
echo "  - high-frequency      Aggressive trading simulation"
echo "  - user-journey        Complete user workflow"
echo "  - report-generation   Heavy reporting load"
echo "  - stress              Progressive stress test"
echo "  - all                 Run all scenarios"
echo ""
echo "For more information, see README.md"
