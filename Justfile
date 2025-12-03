# BrokerX Justfile

set dotenv-load := true

# Show all commands
default:
    @just --list

# ========== DOCKER COMMANDS ==========

# Start entire stack
up:
    @echo "Starting BrokerX stack..."
    docker compose up -d
    @echo "All services started!"
    @echo ""
    @echo "Services:"
    @echo "  Web App:     http://localhost:8000"
    @echo "  API Gateway: http://localhost:8080"
    @echo "  Swagger UI:  http://localhost:8080/swagger-ui/"
    @echo "  Prometheus:  http://localhost:9090"
    @echo "  Grafana:     http://localhost:3000 (admin/admin123)"

# Stop all containers
down:
    @echo "Stopping all containers..."
    docker compose down

# Stop and remove all containers, networks, and volumes
clean:
    @echo "Cleaning up all containers, networks, and volumes..."
    docker compose down -v

# Rebuild and restart all services
rebuild:
    @echo "Rebuilding all services..."
    docker compose up -d --build

# Rebuild specific service
rebuild-service service:
    @echo "Rebuilding {{service}}..."
    docker compose up -d --build {{service}}

# View logs of all services
logs:
    docker compose logs -f

# View logs of specific service
logs-service service:
    docker compose logs -f {{service}}

# Show status of all containers
ps:
    @docker compose ps

# Restart specific service
restart service:
    @echo "Restarting {{service}}..."
    docker compose restart {{service}}

# ========== LOAD TESTING COMMANDS ==========

# Run load test scenario
load-test scenario:
    #!/usr/bin/env bash
    echo "Running load test: {{scenario}}..."
    mkdir -p load-tests/results
    TIMESTAMP=$(date +"%Y%m%d-%H%M%S")
    cd load-tests && k6 run \
        --out json=results/{{scenario}}-$TIMESTAMP.json \
        --summary-export=results/{{scenario}}-$TIMESTAMP-summary.json \
        scenarios/{{scenario}}.js

# Run all load test scenarios
load-test-all:
    @echo "Running all load test scenarios..."
    cd load-tests && ./run-tests.fish all local



