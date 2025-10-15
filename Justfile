# BrokerX Justfile - Simple service management

set dotenv-load := true

# Show all commands
default:
    @just --list

# === START COMMANDS ===

# Start databases
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

# Start everything (db + services + webapp)
all:
    #!/usr/bin/env bash
    just db
    echo ""
    echo "Now run in separate terminals:"
    echo "  just services"
    echo "  just webapp"

# === STOP COMMANDS ===

# Stop databases
db-stop:
    @echo "Stopping databases..."
    docker compose stop user-db order-db market-data-db

# Stop all (only works for docker)
stop:
    @echo "Stopping databases..."
    docker compose stop
