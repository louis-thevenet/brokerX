#!/usr/bin/env fish

# BrokerX k6 Load Testing Runner
# Usage: ./run-tests.fish [scenario] [environment]

set -l SCENARIOS market-data high-frequency user-journey report-generation stress all
set -l ENVIRONMENTS local docker production

# Default values
set -l scenario market-data
set -l environment local
set -l base_url "http://localhost:8080"

# Parse arguments
if test (count $argv) -ge 1
    set scenario $argv[1]
end

if test (count $argv) -ge 2
    set environment $argv[2]
end

# Set base URL based on environment
switch $environment
    case local
        set base_url "http://localhost:8080"
    case docker
        set base_url "http://localhost:8080"
    case production
        echo "⚠️  WARNING: Running against production!"
        set base_url "https://your-production-url.com"
        read -P "Continue? [y/N] " -l confirm
        if test "$confirm" != y
            echo "Aborted."
            exit 1
        end
end

echo "🚀 BrokerX Load Testing"
echo "========================"
echo "Scenario: $scenario"
echo "Environment: $environment"
echo "Base URL: $base_url"
echo ""

# Check if k6 is installed
if not command -v k6 >/dev/null
    echo "❌ Error: k6 is not installed!"
    echo "Install with: brew install k6 (macOS) or see https://k6.io/docs/getting-started/installation/"
    exit 1
end

# Run appropriate scenario
switch $scenario
    case market-data
        echo "📊 Running Market Data Consultation Test..."
        k6 run --out json=results/market-data-$(date +%Y%m%d-%H%M%S).json \
            -e BASE_URL=$base_url \
            scenarios/market-data-consultation.js

    case high-frequency
        echo "⚡ Running High-Frequency Trading Test..."
        k6 run --out json=results/high-frequency-$(date +%Y%m%d-%H%M%S).json \
            -e BASE_URL=$base_url \
            scenarios/high-frequency-orders.js

    case user-journey
        echo "👤 Running Complete User Journey Test..."
        k6 run --out json=results/user-journey-$(date +%Y%m%d-%H%M%S).json \
            -e BASE_URL=$base_url \
            scenarios/complete-user-journey.js

    case all
        echo "🎯 Running All Test Scenarios..."

        for test_scenario in market-data high-frequency user-journey
            echo ""
            echo "Running: $test_scenario"
            ./run-tests.fish $test_scenario $environment

            echo "⏳ Waiting 30 seconds before next test..."
            sleep 30
        end

    case '*'
        echo "❌ Unknown scenario: $scenario"
        echo ""
        echo "Available scenarios:"
        echo "  - market-data         Market data consultation"
        echo "  - high-frequency      High-frequency order placement"
        echo "  - user-journey        Complete user journey"
        echo "  - all                 Run all scenarios"
        echo ""
        echo "Usage: ./run-tests.fish [scenario] [environment]"
        echo "       ./run-tests.fish market-data local"
        echo "       ./run-tests.fish stress docker"
        exit 1
end

echo ""
echo "✅ Test completed!"
