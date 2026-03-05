# Ghost Web

Ghost-branded web UI: landing, customer sign up/sign in, subscriptions (Razorpay), and admin dashboard.

## Setup

### admin-api (Rust)

1. Copy `.env.example` to `.env` in `admin-api/`
2. Set `DATABASE_URL` (same as scribe-api)
3. Set `ADMIN_SECRET` for JWT signing (optional, has default)
4. For Razorpay subscriptions, set:
   - `RAZORPAY_KEY_ID`, `RAZORPAY_KEY_SECRET`
   - `RAZORPAY_PLAN_STARTER`, `RAZORPAY_PLAN_PRO`, `RAZORPAY_PLAN_POWER` (plan IDs from Razorpay Dashboard)
   - `RAZORPAY_WEBHOOK_SECRET` (from Razorpay Dashboard → Webhooks → your endpoint)

**Webhook URL** (for Razorpay Dashboard): `https://your-admin-api-domain/api/payments/webhook`

```bash
cd admin-api
cargo run
```

Runs on port 6660 by default.

### admin-ui (React)

1. Install dependencies and run:

```bash
cd admin-ui
npm install
npm run dev
```

Runs on port 5174. Proxies `/api` to admin-api (localhost:6660).

## Usage

1. Start scribe-api (for DB)
2. Start admin-api
3. Start admin-ui
4. Open http://localhost:5174

**Pages:**
- `/` – Home (Ghost landing)
- `/login` – Sign in (Customer or Admin)
- `/signup` – Sign up (14-day trial)
- `/subscriptions` – Subscription plans (Razorpay)
- `/account` – Customer account (license, upgrade)
- `/dashboard` – Admin analytics (admin only)

**Admin login:** admin / ghostadmin123 (from admin_users table)

## Owner License

The app owner has a permanent license with full access:

- **License key:** `GHOST-OWNER-00000000` (created by migration 005)
- **Full access:** Unlimited tokens, power plan models, no expiry
- **No reset:** Token usage never resets
- **No autopay:** Not linked to Razorpay; payment flow never updates owner

To use your existing license key instead:

```sql
UPDATE licenses SET license_key = 'YOUR-EXISTING-KEY' WHERE is_owner = true;
UPDATE licenses SET user_id = (SELECT id FROM users WHERE is_owner = true) WHERE is_owner = true;
```

Then use `YOUR-EXISTING-KEY` in the Ghost desktop app.
