#!/usr/bin/env fish

# High-Throughput Quality Validation Test
# Target: 800 orders/second sustained, peak 1000 orders/second
# This test validates the performance fixes for the order-execution-orchestrator-service

set -l script_dir (dirname (status -f))

echo "=================================================="
echo "High-Throughput Order Processing Test"
echo "=================================================="
echo "Target: 800 orders/second sustained"
echo "Peak:   1000 orders/second"
echo "Duration: ~12 minutes total"
echo "Users: 50 test users with \$500k each"
echo "=================================================="
echo ""

# Check if k6 is installed
if not type -q k6
    echo "Error: k6 is not installed"
    echo "Install with: brew install k6 (macOS) or see https://k6.io/docs/getting-started/installation/"
    exit 1
end

# Check if services are running
echo "Checking if services are running..."
set -l api_gateway_url "http://localhost:8080/health"
if not curl -s -f $api_gateway_url > /dev/null 2>&1
    echo "Warning: API Gateway may not be running at localhost:8080"
    echo "Make sure to start your services with: docker-compose up"
    read -P "Continue anyway? [y/N] " -n 1 response
    if test "$response" != "y" -a "$response" != "Y"
        exit 1
    end
end

# Set environment variables
set -x BASE_URL "http://localhost:8080"

# Create results directory
set -l timestamp (date +%Y%m%d-%H%M%S)
set -l results_dir "$script_dir/results"
mkdir -p $results_dir

# Run the high-throughput test
echo ""
echo "Starting high-throughput test..."
echo "Results will be saved to: $results_dir/high-throughput-$timestamp.json"
echo ""

k6 run \
    --out json="$results_dir/high-throughput-$timestamp.json" \
    --summary-export="$results_dir/high-throughput-$timestamp-summary.json" \
    "$script_dir/scenarios/quality-validation.js"

set -l exit_code $status

echo ""
echo "=================================================="
if test $exit_code -eq 0
    echo "✓ Test completed successfully!"
else
    echo "✗ Test failed with exit code: $exit_code"
end
echo "=================================================="
echo ""
echo "Results saved to:"
echo "  - $results_dir/high-throughput-$timestamp.json"
echo "  - $results_dir/high-throughput-$timestamp-summary.json"
echo ""
echo "Key metrics to check:"
echo "  - http_reqs rate (should be >800 orders/s during sustained phase)"
echo "  - http_req_duration p95 (should be <300ms)"
echo "  - http_req_failed rate (should be <4.5%)"
echo ""

exit $exit_code
