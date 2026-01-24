#!/bin/bash
set -e

echo "🚀 Deploying Scribe API..."

# Variables
APP_DIR="/opt/scribe-api"
REPO_URL="your-git-repo-url"
BRANCH="main"

# Pull latest code
cd $APP_DIR
git pull origin $BRANCH

# Build
cargo build --release

# Run migrations
sqlx migrate run

# Restart service
sudo systemctl restart scribe-api

# Check status
sudo systemctl status scribe-api

echo "✅ Deployment complete!"
