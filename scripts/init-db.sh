#!/bin/bash

# =============================================================================
# PostgreSQL Database Initialization Script
# =============================================================================
# Usage: 
#   chmod +x scripts/init-db.sh
#   ./scripts/init-db.sh
# =============================================================================

set -e

# Configuration (override via environment variables)
DB_HOST=${DB_HOST:-localhost}
DB_PORT=${DB_PORT:-5335}
DB_USER=${DB_USER:-postgres}
DB_PASSWORD=${DB_PASSWORD:-postgres}
DB_NAME=${DB_NAME:-rust_base}

echo "🐘 PostgreSQL Database Initialization"
echo "======================================"
echo "Host: $DB_HOST:$DB_PORT"
echo "Database: $DB_NAME"
echo ""

# Check if psql is available
if ! command -v psql &> /dev/null; then
    echo "❌ psql command not found. Please install PostgreSQL client."
    exit 1
fi

# Export password for psql
export PGPASSWORD=$DB_PASSWORD

# Create database if it doesn't exist
echo "📦 Creating database '$DB_NAME' if not exists..."
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -tc "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 || \
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -c "CREATE DATABASE $DB_NAME"

echo "✅ Database '$DB_NAME' ready!"

# Set DATABASE_URL for SQLx
export DATABASE_URL="postgres://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"
echo ""
echo "📝 DATABASE_URL:"
echo "   $DATABASE_URL"
echo ""

# Run migrations with SQLx
if command -v sqlx &> /dev/null; then
    echo "🔄 Running SQLx migrations..."
    sqlx migrate run
    echo "✅ Migrations completed!"
else
    echo "⚠️  sqlx-cli not found. Install with:"
    echo "   cargo install sqlx-cli --features postgres"
    echo ""
    echo "   Then run: sqlx migrate run"
fi

echo ""
echo "🎉 Database initialization complete!"
echo ""
echo "Add this to your .env file:"
echo "DATABASE_URL=$DATABASE_URL"
