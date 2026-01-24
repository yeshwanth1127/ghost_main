#!/bin/bash
set -e

echo "🔧 Setting up Scribe API on VPS..."

# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y build-essential pkg-config libssl-dev postgresql postgresql-contrib redis-server nginx

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Setup PostgreSQL
sudo -u postgres createuser scribe_user
sudo -u postgres createdb scribe_db
sudo -u postgres psql -c "ALTER USER scribe_user WITH PASSWORD 'change_this_password';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE scribe_db TO scribe_user;"

# Create application user
sudo useradd -r -s /bin/false scribe || true

# Create app directory
sudo mkdir -p /opt/scribe-api
sudo chown scribe:scribe /opt/scribe-api

# Clone repository (update with your repo)
# sudo -u scribe git clone YOUR_REPO_URL /opt/scribe-api

# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Setup nginx
sudo cp /opt/scribe-api/deploy/nginx.conf /etc/nginx/sites-available/scribe-api
sudo ln -s /etc/nginx/sites-available/scribe-api /etc/nginx/sites-enabled/ || true
sudo nginx -t
sudo systemctl restart nginx

# Setup systemd service
sudo cp /opt/scribe-api/deploy/scribe-api.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable scribe-api

echo "✅ VPS setup complete!"
echo "Next steps:"
echo "1. Copy .env file to /opt/scribe-api/"
echo "2. Run migrations: cd /opt/scribe-api && sqlx migrate run"
echo "3. Build: cargo build --release"
echo "4. Start service: sudo systemctl start scribe-api"
