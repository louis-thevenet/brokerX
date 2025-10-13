#!/usr/bin/env bash

# Fix Market Data Database Column Types
# This script updates the database schema to use DOUBLE PRECISION instead of DECIMAL

echo "🔧 Fixing market data database column types..."
echo ""

# Database connection details
DB_HOST=${MARKET_DATA_DB_HOST:-localhost}
DB_PORT=${MARKET_DATA_DB_PORT:-5434}
DB_USER=${MARKET_DATA_DB_USER:-brokerx_user}
DB_NAME=${MARKET_DATA_DB_NAME:-brokerx_market_data}

echo "📊 Database: $DB_NAME"
echo "🖥️  Host: $DB_HOST:$DB_PORT"
echo "👤 User: $DB_USER"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "❌ Error: psql command not found. Please install PostgreSQL client."
    exit 1
fi

# Run the migration
echo "🔄 Altering column types from DECIMAL to DOUBLE PRECISION..."
PGPASSWORD=${MARKET_DATA_DB_PASSWORD:-password123} psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" << 'EOF'
-- Alter the price columns from DECIMAL to DOUBLE PRECISION
ALTER TABLE market_prices 
    ALTER COLUMN price TYPE DOUBLE PRECISION,
    ALTER COLUMN open TYPE DOUBLE PRECISION,
    ALTER COLUMN high TYPE DOUBLE PRECISION,
    ALTER COLUMN low TYPE DOUBLE PRECISION;

-- Verify the changes
\d market_prices
EOF

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Column types updated successfully!"
    echo ""
    echo "📝 Summary:"
    echo "   - price: DECIMAL → DOUBLE PRECISION"
    echo "   - open: DECIMAL → DOUBLE PRECISION"
    echo "   - high: DECIMAL → DOUBLE PRECISION"
    echo "   - low: DECIMAL → DOUBLE PRECISION"
    echo ""
    echo "🚀 You can now restart the market-data-service"
else
    echo ""
    echo "❌ Failed to update column types"
    echo "Please check the database connection and try again"
    exit 1
fi
