#!/usr/bin/env bash

# Docker Database Management for BrokerX Microservices
set -e

# if db are running, restart them

if [ "$(docker compose ps -q user-db)" ] && [ "$(docker compose ps -q order-db)" ] && [ "$(docker compose ps -q market-data-db)" ]; then
    echo "🔄 Restarting existing database containers..."
    docker compose restart user-db order-db market-data-db
else
    echo "🐳 Starting BrokerX Database Infrastructure..."

    # Start databases
    echo "📊 Starting PostgreSQL databases..."
    docker compose up -d user-db order-db market-data-db
fi

# Wait for databases to be ready
echo "⏳ Waiting for databases to be ready..."
docker compose exec user-db bash -c 'until pg_isready -U brokerx_user -d brokerx_users; do echo "Waiting for user-db..."; sleep 2; done'
docker compose exec order-db bash -c 'until pg_isready -U brokerx_user -d brokerx_orders; do echo "Waiting for order-db..."; sleep 2; done'
docker compose exec market-data-db bash -c 'until pg_isready -U brokerx_user -d brokerx_market_data; do echo "Waiting for market-data-db..."; sleep 2; done'

echo "✅ Databases are ready!"
echo ""
echo "📋 Database Information:"
echo "   User DB:        localhost:5432 (brokerx_users)"
echo "   Order DB:       localhost:5433 (brokerx_orders)"
echo "   Market Data DB: localhost:5434 (brokerx_market_data)"
echo "   Username: brokerx_user"
echo "   Password: password123"
echo ""
echo "🔧 Optional: Start pgAdmin with 'docker compose up -d pgadmin'"
echo "   pgAdmin: http://localhost:8080"
echo "   Email: admin@brokerx.local"
echo "   Password: admin123"
