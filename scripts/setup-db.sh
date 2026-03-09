#!/usr/bin/env bash
# =============================================================================
# Ghost Database Setup Script
# Creates DB, runs scribe-api migrations (includes admin_users, razorpay, etc.)
# Admin-api uses the same schema - no separate migrations.
#
# Usage:
#   ./scripts/setup-db.sh
#   DATABASE_URL=postgresql://user:pass@host:5432/ghost ./scripts/setup-db.sh
#   PG_USER=postgres PG_PASSWORD=xxx PG_DATABASE=ghost ./scripts/setup-db.sh
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCRIBE_API_DIR="$REPO_ROOT/scribe/scribe/scribe-api"

# Load DATABASE_URL from scribe-api/.env if not set
if [[ -z "$DATABASE_URL" && -f "$SCRIBE_API_DIR/.env" ]]; then
    export DATABASE_URL=$(grep '^DATABASE_URL=' "$SCRIBE_API_DIR/.env" | cut -d= -f2-)
fi

# Require DATABASE_URL
if [[ -z "$DATABASE_URL" ]]; then
    echo "Error: DATABASE_URL not set. Set it or add to scribe/scribe/scribe-api/.env"
    exit 1
fi

# Parse DATABASE_URL for psql (user:pass@host:port/dbname)
# Use a temp DB URL for postgres system DB (for CREATE DATABASE)
DB_FOR_CREATE="${DATABASE_URL%/ghost}/postgres"
DB_NAME="ghost"
if [[ "$DATABASE_URL" =~ /([^/]+)$ ]]; then
    DB_NAME="${BASH_REMATCH[1]}"
fi

echo "=============================================="
echo "Ghost Database Setup"
echo "=============================================="
echo "Database: $DB_NAME"
echo "=============================================="

# Step 1: Create database if it doesn't exist
echo ""
echo "[1/2] Creating database '$DB_NAME' (if not exists)..."
psql "$DB_FOR_CREATE" -tc "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 \
    || psql "$DB_FOR_CREATE" -c "CREATE DATABASE $DB_NAME;"
echo "  Done."

# Step 2: Run scribe-api migrations (creates all tables + seed data)
echo ""
echo "[2/2] Running migrations (scribe-api)..."
cd "$SCRIBE_API_DIR"
sqlx migrate run
echo "  Done."

echo ""
echo "=============================================="
echo "Setup complete!"
echo "=============================================="
echo ""
echo "Tables created:"
psql "$DATABASE_URL" -c "\dt" 2>/dev/null || true
echo ""
echo "Default admin: username=admin, password=ghostadmin123"
echo "Owner license: GHOST-OWNER-00000000"
echo ""
