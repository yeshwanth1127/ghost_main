# Scribe API

Production-ready Rust API for Scribe AI Assistant.

## Architecture

- **Framework**: Axum (Rust)
- **Database**: PostgreSQL
- **Caching**: Redis
- **AI Routing**: OpenRouter
- **STT**: OpenAI Whisper

## Setup

### Prerequisites

- Rust 1.75+
- PostgreSQL 15+
- Redis 7+

### Installation

1. Clone and navigate:
```bash
cd Scribe/scribe-api
```

2. Copy environment file:
```bash
cp .env.example .env
```

3. Update `.env` with your API keys

4. Install PostgreSQL and create database:
```bash
createdb scribe_db
```

5. Run migrations:
```bash
cargo install sqlx-cli
sqlx migrate run
```

6. Build and run:
```bash
cargo build --release
cargo run --release
```

## API Endpoints

### Health
- `GET /health` - Health check
- `GET /api/v1/status` - API status

### Authentication
- `POST /api/v1/activate` - Activate license
- `POST /api/v1/deactivate` - Deactivate license
- `POST /api/v1/validate` - Validate license
- `GET /api/v1/checkout` - Get checkout URL

### AI Services
- `POST /api/v1/chat` - Chat completion (SSE streaming)
- `POST /api/v1/audio` - Speech-to-text transcription
- `POST /api/v1/models` - List available models
- `POST /api/v1/prompt` - Generate system prompt

## Deployment

### VPS Deployment

1. Install dependencies on VPS:
```bash
sudo apt update
sudo apt install -y build-essential postgresql postgresql-contrib redis-server
```

2. Setup PostgreSQL:
```bash
sudo -u postgres createuser scribe_user
sudo -u postgres createdb scribe_db
sudo -u postgres psql -c "ALTER USER scribe_user WITH PASSWORD 'secure_password';"
```

3. Clone and build:
```bash
git clone <your-repo>
cd scribe-api
cargo build --release
```

4. Setup systemd service:
```bash
sudo cp deploy/scribe-api.service /etc/systemd/system/
sudo systemctl enable scribe-api
sudo systemctl start scribe-api
```

5. Setup nginx reverse proxy:
```bash
sudo cp deploy/nginx.conf /etc/nginx/sites-available/scribe-api
sudo ln -s /etc/nginx/sites-available/scribe-api /etc/nginx/sites-enabled/
sudo systemctl restart nginx
```

## Development

```bash
cargo watch -x run
```

## Testing

```bash
cargo test
```
