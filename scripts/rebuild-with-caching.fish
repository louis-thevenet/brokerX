#!/usr/bin/env fish

# Rebuild and restart services with caching improvements
echo "🔨 Rebuilding services with price caching optimizations..."
echo ""

set -l services_to_rebuild api-gateway order-execution-orchestrator-service

for service in $services_to_rebuild
    echo "📦 Building $service..."
    docker build -f docker/Dockerfile.$service -t brokerx-$service . 2>&1 | grep -E "(Step|Successfully|ERROR)" || true
    echo ""
end

echo "🔄 Restarting services..."
docker restart brokerx-api-gateway brokerx-order-execution-orchestrator-service

echo ""
echo "⏳ Waiting for services to be ready..."
sleep 5

echo ""
echo "✅ Services rebuilt and restarted!"
echo ""
echo "📊 Check logs:"
echo "  docker logs -f brokerx-order-execution-orchestrator-service"
echo "  docker logs -f brokerx-api-gateway"
echo ""
echo "🧪 Test the setup:"
echo "  curl http://localhost:8080/v1/market/prices/AAPL"
echo ""
echo "🚀 Run performance test:"
echo "  cd load-tests && ./run-high-throughput-test.fish"
