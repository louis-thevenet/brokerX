# BrokerX Justfile - Simple service management

set dotenv-load := true

# Show all commands
default:
    @just --list

# ========== DOCKER COMMANDS ==========

# Start entire stack (databases, services, monitoring)
docker-up:
    @echo "🚀 Starting entire BrokerX stack..."
    docker compose up -d
    @echo ""
    @echo "✅ All services started!"
    @echo ""
    @echo "📊 Services:"
    @echo "  Web App:            http://localhost:8000"
    @echo "  API Gateway:        http://localhost:8080"
    @echo "  Swagger UI:         http://localhost:8080/swagger-ui/"
    @echo ""
    @echo "🔍 Monitoring:"
    @echo "  Prometheus:         http://localhost:9090"
    @echo "  Grafana:            http://localhost:3000 (admin/admin123)"
    @echo ""
    @echo "🗄️  Databases:"
    @echo "  User DB:            localhost:5432"
    @echo "  Order DB:           localhost:5433"
    @echo "  Market Data DB:     localhost:5434"
    @echo ""
    @echo "Run 'just docker-logs' to see logs"

# Start only databases
docker-db:
    @echo "🗄️  Starting databases..."
    docker compose up -d user-db order-db market-data-db
    @echo "✅ Databases started!"

# Start only microservices (assumes databases are running)
docker-services:
    @echo "🚀 Starting microservices and webapp..."
    docker compose up -d user-service order-service market-data-service order-execution-orchestrator-service api-gateway webapp
    @echo "✅ Microservices and webapp started!"

# Start only monitoring stack
docker-monitoring:
    @echo "📊 Starting monitoring stack..."
    docker compose up -d prometheus grafana
    @echo "✅ Monitoring started!"
    @echo "  Prometheus: http://localhost:9090"
    @echo "  Grafana:    http://localhost:3000 (admin/admin123)"

# Rebuild and restart all services
docker-rebuild:
    @echo "🔨 Rebuilding and restarting all services..."
    docker compose up -d --build
    @echo "✅ Services rebuilt and restarted!"

# Rebuild and restart a specific service
docker-rebuild-service service:
    @echo "🔨 Rebuilding {{service}}..."
    docker compose up -d --build {{service}}
    @echo "✅ {{service}} rebuilt and restarted!"

# Stop all containers
docker-down:
    @echo "🛑 Stopping all containers..."
    docker compose down
    @echo "✅ All containers stopped!"

# Stop and remove all containers, networks, and volumes
docker-clean:
    @echo "🧹 Cleaning up all containers, networks, and volumes..."
    docker compose down -v
    @echo "✅ Cleanup complete!"

# View logs of all services
docker-logs:
    docker compose logs -f

# View logs of a specific service
docker-logs-service service:
    docker compose logs -f {{service}}

# Show status of all containers
docker-ps:
    @docker compose ps

# Restart a specific service
docker-restart service:
    @echo "🔄 Restarting {{service}}..."
    docker compose restart {{service}}
    @echo "✅ {{service}} restarted!"

# Execute a command in a running container
docker-exec service cmd:
    docker compose exec {{service}} {{cmd}}

# Open a shell in a running container
docker-shell service:
    docker compose exec {{service}} /bin/bash

# Start with pgadmin for database management
docker-up-tools:
    @echo "🚀 Starting stack with database tools..."
    docker compose --profile tools up -d
    @echo "✅ All services started!"
    @echo "  pgAdmin: http://localhost:5050 (admin@brokerx.local/admin123)"

# ========== LOCAL DEVELOPMENT COMMANDS ==========

# Start databases (for local development)
db:
    #!/usr/bin/env bash
    echo "Starting databases..."
    docker compose up -d user-db order-db market-data-db
    echo "Waiting for databases..."
    sleep 3
    echo "Databases ready!"

# Start user service (log level: error, warn, info, debug, trace)
user log="info":
    @echo "Starting User Service (RUST_LOG={{log}})..."
    RUST_LOG={{log}} cargo run --release -p user-service

# Start order service (log level: error, warn, info, debug, trace)
order log="info":
    @echo "Starting Order Service (RUST_LOG={{log}})..."
    RUST_LOG={{log}} cargo run --release -p order-service

# Start market data service (log level: error, warn, info, debug, trace)
market log="info":
    @echo "Starting Market Data Service (RUST_LOG={{log}})..."
    RUST_LOG={{log}} cargo run --release -p market-data-service

# Start API gateway (log level: error, warn, info, debug, trace)
gateway log="info":
    @echo "Starting API Gateway (RUST_LOG={{log}})..."
    RUST_LOG={{log}} cargo run --release -p api_gateway

# Start web application
webapp:
    #!/usr/bin/env bash
    cd webapp
    echo "Starting Web App on http://localhost:3000"
    python3 -m http.server 3000

# Start all microservices together (log level: error, warn, info, debug, trace)
services log="info":
    #!/usr/bin/env bash
    echo "Building and starting all services (RUST_LOG={{log}})..."
    cargo build --release -p user-service -p order-service -p market-data-service -p api_gateway
    
    echo "Starting services in background..."
    RUST_LOG={{log}} cargo run --release -p user-service &
    USER_PID=$!
    
    RUST_LOG={{log}} cargo run --release -p order-service &
    ORDER_PID=$!
    
    RUST_LOG={{log}} cargo run --release -p market-data-service &
    MARKET_PID=$!

    RUST_LOG={{log}} cargo run --release -p order-execution-orchestrator-service &
    MARKET_PID=$!
    
    RUST_LOG={{log}} cargo run --release -p api_gateway &
    GATEWAY_PID=$!
    
    echo ""
    echo "All services running!"
    echo "API Gateway: http://localhost:8080"
    echo "Swagger UI: http://localhost:8080/swagger-ui/"
    echo ""
    echo "Press Ctrl+C to stop..."
    
    cleanup() {
        echo ""
        echo "Stopping services..."
        kill $USER_PID $ORDER_PID $MARKET_PID $GATEWAY_PID 2>/dev/null
        exit 0
    }
    
    trap cleanup INT TERM
    wait

# Start everything locally (db + services + webapp)
all:
    #!/usr/bin/env bash
    just db
    echo ""
    echo "Now run in separate terminals:"
    echo "  just services"
    echo "  just webapp"

# Stop databases
db-stop:
    @echo "Stopping databases..."
    docker compose stop user-db order-db market-data-db

# Stop all docker services
stop:
    @echo "Stopping all docker services..."
    docker compose stop


# Show help
help:
    @echo "BrokerX - Microservice Trading Platform"
    @echo ""
    @echo "🐳 Docker Commands (Recommended for full stack):"
    @echo "  just docker-up              - Start entire stack"
    @echo "  just docker-down            - Stop all containers"
    @echo "  just docker-logs            - View all logs"
    @echo "  just docker-rebuild         - Rebuild and restart all"
    @echo ""
    @echo "💻 Local Development Commands:"
    @echo "  just db                     - Start databases only"
    @echo "  just services               - Run all services locally"
    @echo "  just user                   - Run user service"
    @echo "  just order                  - Run order service"
    @echo "  just market                 - Run market data service"
    @echo "  just gateway                - Run API gateway"
    @echo ""
    @echo "🧪 Load Testing Commands:"
    @echo "  just load-test-setup        - Setup k6 load testing"
    @echo "  just load-test [scenario]   - Run load tests"
    @echo "  just load-test-all          - Run all load test scenarios"
    @echo ""
    @echo "✅ Quality Validation:"
    @echo "  just quality-test           - Validate quality objectives"
    @echo "  just quality-metrics        - View metrics in Prometheus"
    @echo ""
    @echo "For full list: just --list"

# ========== LOAD TESTING COMMANDS ==========

# Setup k6 load testing environment
load-test-setup:
    @echo "🧪 Setting up load testing..."
    cd load-tests && ./setup.fish && ./test-mfa-bypass.fish

# Run load tests (market-data, high-frequency, user-journey, quality-validation, stress, all)
load-test scenario="market-data":
    #!/usr/bin/env bash
    echo "🧪 Running load test: {{scenario}}..."
    mkdir -p load-tests/results
    TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
    cd load-tests && k6 run \
        --out json=results/{{scenario}}-$TIMESTAMP.json \
        --summary-export=results/{{scenario}}-$TIMESTAMP-summary.json \
        scenarios/{{scenario}}.js

# Run all load test scenarios
load-test-all:
    @echo "🧪 Running all load test scenarios..."
    cd load-tests && ./run-tests.fish all local

# Run smoke test (quick validation)
load-test-smoke:
    @echo "🧪 Running smoke test..."
    @mkdir -p load-tests/results
    @TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
    cd load-tests && k6 run \
        --vus 1 --duration 30s \
        --out json=results/smoke-$TIMESTAMP.json \
        --summary-export=results/smoke-$TIMESTAMP-summary.json \
        -e BASE_URL=http://localhost:8080 \
        scenarios/market-data-consultation.js

# Run quality validation test (validates all 3 quality objectives)
quality-test:
    @echo "================================================================================"
    @echo "BrokerX Quality Validation Test"
    @echo "================================================================================"
    @echo ""
    @echo "Quality Objectives:"
    @echo "  1. Availability: ≥ 95.5%"
    @echo "  2. Performance: Latency (order → ACK) ≤ 250ms (p95)"
    @echo "  3. Performance: Throughput ≥ 800 orders/s"
    @echo "  4. Observability: Metrics available in Prometheus/Grafana"
    @echo ""
    @echo "================================================================================"
    @echo ""
    @echo "Checking services..."
    @curl -sf http://localhost:8080/v1/health > /dev/null || (echo "❌ API Gateway is not running! Run: just docker-up" && exit 1)
    @echo "✅ API Gateway is running"
    @curl -sf http://localhost:9090/-/healthy > /dev/null && echo "✅ Prometheus is running" || echo "⚠️  Prometheus is not running"
    @curl -sf http://localhost:3000/api/health > /dev/null && echo "✅ Grafana is running" || echo "⚠️  Grafana is not running"
    @echo ""
    @echo "================================================================================"
    @echo "Starting Quality Validation Test..."
    @echo "================================================================================"
    @echo ""
    @mkdir -p load-tests/results
    @TIMESTAMP=$(date +"%Y%m%d-%H%M%S") && \
    cd load-tests && k6 run \
        --out json=results/quality-validation-$$TIMESTAMP.json \
        --summary-export=results/quality-validation-$$TIMESTAMP-summary.json \
        scenarios/quality-validation.js && \
    echo "" && \
    echo "================================================================================" && \
    echo "✅ Quality validation PASSED!" && \
    echo "" && \
    echo "Observability Check:" && \
    echo "  - Prometheus metrics: http://localhost:9090/graph" && \
    echo "  - Grafana dashboards: http://localhost:3000" && \
    echo "  - Service logs: just docker-logs" && \
    echo "================================================================================" || \
    (echo "" && \
    echo "================================================================================" && \
    echo "❌ Quality validation FAILED!" && \
    echo "" && \
    echo "Check the output above for failed thresholds." && \
    echo "Debug with: just docker-logs or just docker-logs-service <service>" && \
    echo "================================================================================" && \
    exit 1)

# View quality metrics in Prometheus
quality-metrics:
    @echo "📊 Opening Prometheus for quality metrics..."
    @echo ""
    @echo "Key metrics to check:"
    @echo "  - http_requests_total (Traffic)"
    @echo "  - http_request_duration_seconds (Latency)"
    @echo "  - http_requests_failed (Errors)"
    @echo "  - process_cpu_seconds_total (Saturation)"
    @echo ""
    @echo "Prometheus: http://localhost:9090/graph"
    @echo "Grafana:    http://localhost:3000"




