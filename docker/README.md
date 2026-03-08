# Ghost Local Docker Setup

Run scribe-api, PostgreSQL, and Redis locally. The web UI and admin-api are hosted at **ghost.exora.solutions**.

## Quick Start

```bash
# 1. Copy env and set required secrets
cp docker/.env.example .env
# Edit .env: set API_ACCESS_KEY, OPENROUTER_API_KEY, OPENAI_API_KEY

# 2. Start services
docker compose up -d

# 3. Run migrations (one-time)
docker compose exec scribe-api sqlx migrate run

# 4. Scribe desktop app: set in scribe/scribe/.env
#    APP_ENDPOINT=http://127.0.0.1:8083
#    PAYMENT_ENDPOINT=http://127.0.0.1:8083
#    API_ACCESS_KEY=<same as .env>
```

## Services

| Service    | Port | Purpose                          |
|-----------|------|----------------------------------|
| scribe-api| 8083 | Chat, license activation, usage |
| postgres  | 5432 | Database                         |
| redis     | 6379 | Cache                            |

## Web UI (Hosted)

- Registration, subscriptions: https://ghost.exora.solutions
- Admin dashboard: https://ghost.exora.solutions/dashboard

Checkout redirects from Scribe go to ghost.exora.solutions (configured via `PAYMENT_BASE_URL` in docker-compose).

## Shared Database

To use licenses created at ghost.exora.solutions with local Scribe, set `DATABASE_URL` in `.env` to the production DB used by admin-api.

## Production Deployment

For deploying scribe-api on your server and exposing it at api.ghost.exora.solutions, see [docs/DEPLOY.md](../docs/DEPLOY.md). Override in docker-compose or pass as env to scribe-api.
