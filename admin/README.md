# Ghost Admin Dashboard

Admin-only analytics dashboard for Ghost/Scribe. Separate backend (admin-api) and frontend (admin-ui).

## Setup

### admin-api (Rust)

1. Copy `.env.example` to `.env` in `admin-api/`
2. Set `DATABASE_URL` (same as scribe-api)
3. Set `ADMIN_USERNAME` and `ADMIN_PASSWORD` (or `ADMIN_PASSWORD_HASH`)
4. Set `ADMIN_SECRET` for JWT signing

```bash
cd admin-api
cargo run
```

Runs on port 8084 by default.

### admin-ui (React)

1. Install dependencies and run:

```bash
cd admin-ui
npm install
npm run dev
```

Runs on port 5174. Proxies `/api` to admin-api (localhost:8084).

## Usage

1. Start scribe-api (for DB)
2. Start admin-api
3. Start admin-ui
4. Open http://localhost:5174
5. Log in with ADMIN_USERNAME / ADMIN_PASSWORD
