# Licensing, Authentication & Subscription Model

This document explains how licensing, authentication, and subscriptions work in the Ghost/Scribe application, including verification flows, license generation, and subscription management.

---

## Table of Contents

1. [Overview](#overview)
2. [License Model](#license-model)
3. [License Key Generation](#license-key-generation)
4. [License Verification & Activation](#license-verification--activation)
5. [Authentication Systems](#authentication-systems)
6. [Subscription Model](#subscription-model)
7. [Usage Tracking & Token Limits](#usage-tracking--token-limits)
8. [Data Flow Diagrams](#data-flow-diagrams)

---

## Overview

The application has three main components:

| Component | Purpose |
|-----------|---------|
| **Scribe API** (`scribe-api`) | Backend for the desktop app: license activation, chat, usage tracking |
| **Admin API** (`admin-api`) | Admin dashboard & payment processing (Razorpay) |
| **Admin UI** | Web interface for subscriptions, payments, customer management |

Licenses are **device-bound** (machine ID) and support **multiple instances per license** (configurable `max_instances`). Users can have **trial**, **free**, or **paid** plans with different token limits.

---

## License Model

### License Types

| Type | Format | Description |
|------|--------|--------------|
| **Trial** | `TRIAL-{UUID}` | 14-day trial, created on first launch |
| **Paid/Free** | `GHOST-XXXXXXXX-XXXXXXXX` | 8+8 alphanumeric chars |
| **Owner** | `GHOST-OWNER-00000000` | Special license: no expiry, unlimited tokens |

### License Tiers

| Tier | Token Limit | Notes |
|------|-------------|-------|
| `free` | 5,000/month | Default for new users |
| `starter` | 500,000/month | Razorpay plan |
| `pro` | 1,000,000/month | Razorpay plan |
| `power` | 2,000,000/month | Razorpay plan |
| Owner | Unlimited | `is_owner = true` |

### Database Schema (licenses)

```sql
licenses (
  id, license_key, user_id, status, tier, max_instances,
  is_trial, trial_ends_at, expires_at, is_owner, created_at, updated_at
)
```

- **status**: `active`, `suspended`, `expired`
- **max_instances**: Number of devices (machine IDs) that can activate
- **is_owner**: Owner licenses skip expiry, trial checks, and token limits

---

## License Key Generation

### How Keys Are Generated

License keys are generated server-side using a deterministic format:

**Paid/Free keys** (`scribe-api`, `admin-api`):

```rust
// Format: GHOST-XXXXXXXX-XXXXXXXX
fn generate_license_key() -> String {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let part1 = random_8_chars();
    let part2 = random_8_chars();
    format!("GHOST-{}-{}", part1, part2)
}
```

**Trial keys** (client-side, Tauri):

```rust
// Format: TRIAL-{UUID}
let trial_license_key = format!("TRIAL-{}", Uuid::new_v4());
```

### When Keys Are Created

| Scenario | Where | Format |
|----------|-------|--------|
| User registration (scribe-api) | `POST /api/v1/auth/register` | `GHOST-*` |
| User registration (admin-api) | `POST /api/auth/register` | `GHOST-*` |
| Razorpay payment verified | `admin-api` payments flow | `GHOST-*` |
| First launch trial | `create_trial_license` Tauri command → `POST /api/v1/create-trial` | `TRIAL-*` |

---

## License Verification & Activation

### 1. Activation Flow (Desktop App)

When the user enters a license key or starts a trial:

```
┌─────────────────┐     POST /api/v1/activate      ┌─────────────────┐
│  Scribe (Tauri)  │ ──────────────────────────────►│   Scribe API     │
│                  │  Authorization: Bearer {key}  │                  │
│  - license_key   │  Body: { license_key,          │  1. Check license│
│  - machine_id    │        instance_name,          │     exists &     │
│  - instance_name │        machine_id, app_version │     status=active│
│  - app_version   │  }                             │  2. Insert       │
│                  │                                │     license_     │
│                  │◄────────────────────────────── │     instances    │
│  Store in        │  { activated, instance }       │     (or update)  │
│  secure_storage  │                                │  3. Return       │
└─────────────────┘                                └─────────────────┘
```

**Endpoints:**

- `POST /api/v1/activate` — Activate a license on this machine
- `POST /api/v1/deactivate` — Remove this machine from the license
- `POST /api/v1/validate` — Check if license is still active (trial expiry, paid expiry)
- `POST /api/v1/create-trial` — Create and activate a 14-day trial

**Client credentials** (Tauri):

- `PAYMENT_ENDPOINT` — Base URL of scribe-api (e.g. `https://ghost.exora.solutions`)
- `API_ACCESS_KEY` — Bearer token sent in `Authorization` header (must match server `API_ACCESS_KEY`)

### 2. Validation Logic

The `validate` endpoint checks:

1. License exists in DB
2. `status == 'active'`
3. If **not** owner:
   - Trial: `trial_ends_at > now`
   - Paid: `expires_at` (if set) > now
4. Updates `last_validated_at` on the instance

### 3. Instance Binding

Each activation creates a `license_instances` row:

- `(license_id, machine_id)` is **unique** — one machine per license
- `machine_id` comes from `tauri_plugin_machine_uid` (hardware fingerprint)
- `instance_name` is a UUID generated per activation

---

## Authentication Systems

### 1. Scribe API (Desktop & Chat)

**License-based (no password):**

- Chat: `x-license-key` or `license-key` header → resolves `user_id` from DB
- Usage recording: same header
- Activation/validate: no auth required (client sends `Authorization: Bearer {API_ACCESS_KEY}` but server may not enforce it on all routes)

**User identity:**

- `POST /api/v1/auth/register` — Email only, creates user + license with 14-day trial
- `POST /api/v1/auth/login` — Email only, returns `user_id`, `license_key`, `plan`
- `POST /api/v1/auth/get-user` — Get user info from `license_key`

### 2. Admin API (Dashboard)

**Admin users** (`admin_users` table):

- Login: `POST /api/auth/login` with `username` + `password`
- bcrypt password hash
- JWT with `sub` = username, `exp` = 7 days
- Protected routes: `Authorization: Bearer {token}`

**Customer users** (for admin UI customer portal):

- Login: `POST /api/auth/customer-login` with `email` + `password`
- bcrypt, JWT with `sub` = user_id, `email`, `exp` = 7 days
- Used for customer-facing subscription management

### 3. Admin API License Generation

- `generate_license_key()` in `admin-api` creates `GHOST-*` keys
- Used when: new user registration, Razorpay payment verification (if no license exists)

---

## Subscription Model

### Payment Provider: Razorpay

Subscriptions are managed via **Razorpay** (Indian payment gateway).

### Plans

| Plan | Razorpay Plan ID | Token Limit | Notes |
|------|------------------|-------------|-------|
| starter | `config.razorpay_plan_starter` | 500,000 | 12-month subscription |
| pro | `config.razorpay_plan_pro` | 1,000,000 | 12-month subscription |
| power | `config.razorpay_plan_power` | 2,000,000 | 12-month subscription |

### Subscription Flow

```
┌──────────────┐    create_subscription     ┌──────────────┐    Razorpay Checkout    ┌──────────────┐
│  Admin UI    │ ──────────────────────────►│  Admin API   │ ──────────────────────►│  Razorpay    │
│  (customer)  │  plan, email, user_id?     │              │  subscription_id,       │              │
│              │                            │  Returns     │  key_id                 │  Payment     │
│              │                            │  sub_id,     │                         │  UI          │
│              │                            │  key_id      │                         │              │
└──────────────┘                            └──────────────┘                         └──────────────┘
        │                                           │                                        │
        │                                           │                                        │
        │              verify_payment                │         Webhook                       │
        │  ◄────────────────────────────────────────│  subscription.charged               │
        │  razorpay_payment_id,                      │  subscription.cancelled              │
        │  razorpay_subscription_id,                 │  subscription.completed              │
        │  razorpay_signature                        │  subscription.halted                 │
        │                                            │                                        │
        │  HMAC-SHA256 signature verification       │  X-Razorpay-Signature                 │
        │  payload: payment_id|subscription_id       │  HMAC-SHA256(body, webhook_secret)    │
        └────────────────────────────────────────────┴────────────────────────────────────────┘
```

### Verification

1. **Client-side:** Razorpay returns `payment_id`, `subscription_id`, `signature` after payment
2. **Server:** `verify_razorpay_signature(secret, payment_id, subscription_id, signature)`
   - Payload: `{payment_id}|{subscription_id}`
   - HMAC-SHA256 with `razorpay_key_secret`
3. Fetch subscription from Razorpay API to get `plan_id`, `notes` (email, user_id)
4. Create/update user + license in DB, set `plan`, `monthly_token_limit`, `razorpay_subscription_id`

### Webhooks

- `subscription.charged` — Log transaction
- `subscription.cancelled` / `subscription.completed` / `subscription.halted` — Downgrade to `free`, 5,000 tokens, clear `razorpay_subscription_id`

---

## Usage Tracking & Token Limits

### How Usage Is Recorded

1. **Chat API** (`POST /api/v1/chat`):
   - Header: `x-license-key` → resolve `user_id`
   - Before streaming: `check_token_limit(user_id, 0)`
   - After stream: extract `prompt_tokens`, `completion_tokens` from OpenRouter response
   - Call `record_usage_from_client(UsageRecord)`

2. **Direct recording** (`POST /api/v1/usage/record`):
   - Header: `x-license-key`
   - Body: `{ model, provider, prompt_tokens, completion_tokens }`

### Token Limit Check

```rust
// usage_service.check_token_limit(user_id, requested_tokens)
// - Owner: always allowed, unlimited
// - Else: tokens_used + requested <= monthly_token_limit
// - Warning at 90% usage
// - allowed = false if exceeded
```

### Monthly Reset

- `users.monthly_reset_at` — When to reset `tokens_used_this_month`
- DB trigger: `reset_monthly_tokens` on `users` UPDATE
- Background task: hourly job resets users where `monthly_reset_at < NOW()`

### Plan Limits (Recap)

| Plan | Monthly Tokens |
|------|----------------|
| free | 5,000 |
| starter | 500,000 |
| pro | 1,000,000 |
| power | 2,000,000 |
| owner | Unlimited |

---

## Data Flow Diagrams

### First Launch (Trial)

```
User opens app (no license)
    → is_first_launch() = true
    → create_trial_license()
        → Generate TRIAL-{UUID}
        → POST /api/v1/create-trial (Bearer API_ACCESS_KEY)
        → Server: INSERT licenses (trial, 14 days), INSERT license_instances
        → Store license_key + instance_id in secure_storage.json
```

### Paid Upgrade

```
User on trial/free
    → Clicks "Upgrade" → Admin UI subscriptions page
    → createSubscription(plan, email, user_id?, license_key?)
    → Razorpay checkout
    → Payment success → verify_payment(payment_id, sub_id, signature)
    → Server: UPDATE users SET plan, monthly_token_limit, razorpay_subscription_id
              UPDATE licenses SET tier, is_trial=false
              (or INSERT user+license if new)
    → Return license_key to client
```

### Chat Request

```
Desktop sends chat request
    → Header: x-license-key (from secure_storage)
    → Scribe API: get_user_id_from_license(license_key)
    → check_token_limit(user_id) → if exceeded: 402 Payment Required
    → model_router.route_model(user_id, ...) → plan-based model selection
    → OpenRouter chat stream
    → On stream end: record_usage_from_client(...)
```

---

## Environment Variables (Relevant)

| Variable | Where | Purpose |
|----------|-------|---------|
| `API_ACCESS_KEY` | scribe-api | Bearer token for activation/validate (client must send) |
| `PAYMENT_ENDPOINT` | Tauri (build) | Base URL of scribe-api |
| `DATABASE_URL` | scribe-api, admin-api | PostgreSQL connection |
| `ADMIN_SECRET` | admin-api | JWT signing secret for admin/customer tokens |
| `RAZORPAY_KEY_ID` | admin-api | Razorpay API key |
| `RAZORPAY_KEY_SECRET` | admin-api | Razorpay API secret |
| `RAZORPAY_WEBHOOK_SECRET` | admin-api | Webhook signature verification |
| `RAZORPAY_PLAN_STARTER/PRO/POWER` | admin-api | Razorpay plan IDs |

---

## Summary

- **Licenses** are device-bound (machine_id), support trials and paid tiers.
- **License keys** are generated as `GHOST-XXXXXXXX-XXXXXXXX` or `TRIAL-{UUID}`.
- **Activation** binds a machine to a license; **validation** checks expiry and trial status.
- **Subscriptions** use Razorpay; payment verification is HMAC-signed; webhooks handle renewals/cancellations.
- **Authentication** is split: license-key for scribe API, JWT for admin/customer.
- **Usage** is tracked per user; token limits are enforced before chat; monthly reset is automatic.
