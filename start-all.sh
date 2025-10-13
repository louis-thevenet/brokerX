t#!/usr/bin/env bash

# Complete BrokerX Startup Script
# This script starts databases and all microservices

set -e

echo "🏗️  Starting Complete BrokerX Infrastructure..."
echo ""

# Start databases first
echo "1️⃣  Starting databases..."
sudo ./start-databases.sh

echo ""
echo "2️⃣  Starting web application..."
./webapp/start-webapp.sh &

echo ""
echo "3️⃣2️⃣ Starting microservices..."
./start-microservices.sh

