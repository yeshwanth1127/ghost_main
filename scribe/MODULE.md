# Ghost Scribe Module

A desktop AI assistant built with Tauri, React, and a Rust backend (scribe-api). Supports multiple AI providers (Exora/Ollama, Scribe API via OpenRouter), usage tracking, license management, and agent mode.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Tauri Desktop App                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐  │
│  │   React UI   │  │  Tauri API   │  │  Event Listeners               │  │
│  │  (Vite)      │  │  (api.rs)    │  │  chat_stream_chunk/complete   │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────▲─────────────────┘  │
│         │                 │                         │                    │
│         │  invoke()       │  HTTP/SSE               │  emit_to("main")   │
└─────────┼─────────────────┼─────────────────────────┼────────────────────┘
          │                 │                         │
          ▼                 ▼                         │
┌─────────────────────────────────────────────────────────────────────────┐
│                    scribe-api (Rust, port 8083)                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────────────┐  │
│  │  Chat      │  │  Usage     │  │  Auth      │  │  OpenRouter       │  │
│  │  (SSE)     │  │  Tracking │  │  License   │  │  Service          │  │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └────────┬─────────┘  │
│        │               │               │                   │           │
│        └───────────────┴───────────────┴───────────────────┘           │
│                                    │                                     │
│                          PostgreSQL + Redis                              │
└────────────────────────────────────┼─────────────────────────────────────┘
                                     │
                                     ▼
                          ┌─────────────────────┐
                          │  OpenRouter API     │
                          │  (400+ AI models)   │
                          └─────────────────────┘
```

---

## Directory Structure

```
scribe/
├── MODULE.md                    # This file
├── scribe/                      # Tauri + React frontend
│   ├── src/
│   │   ├── components/         # React UI (completion, settings, agent)
│   │   ├── config/              # Constants, AI providers, capabilities
│   │   ├── contexts/            # App context, provider state
│   │   ├── hooks/               # useCompletion, useApp, etc.
│   │   ├── lib/                 # API clients, functions, storage
│   │   └── types/               # TypeScript types
│   └── src-tauri/               # Tauri Rust backend
│       └── src/
│           ├── api.rs           # chat_stream, HTTP → scribe-api
│           ├── activate.rs     # License activation
│           └── agent/           # Agent mode (Pi)
└── scribe-api/                  # Rust API server
    ├── src/
    │   ├── main.rs              # Entry, routes, CORS
    │   ├── config.rs            # Env config
    │   ├── routes/              # chat, usage, auth, models, audio
    │   ├── services/            # openrouter, usage, model_router
    │   ├── models/              # ChatRequest, UsageRecord, etc.
    │   └── gateway/             # WebSocket for agent mode
    └── migrations/              # PostgreSQL schema
```

---

## AI Provider Modes

| Mode | When Used | Flow |
|------|-----------|------|
| **Exora AI** | User selects Exora + local Ollama | Tauri → Ollama (localhost:11434) directly |
| **Scribe API** | User selects Exora + Scribe model, or Scribe API enabled | Tauri → scribe-api → OpenRouter |
| **Direct (curl)** | User selects OpenAI, Claude, etc. | Tauri → provider API directly via curl |

**Scribe API decision** (`shouldUseScribeAPI`):
- `Scribe_api_enabled` in localStorage = true
- License is active
- Provider is Exora and user has selected a Scribe model, OR provider is Scribe API

---

## Chat Flow (Scribe API)

1. **Frontend** (`useCompletion` / `ai-response.function.ts`): Calls `invoke("chat_stream", {...})`
2. **Tauri** (`api.rs`): Sends POST to `http://127.0.0.1:8083/api/v1/chat?stream=true` with headers:
   - `license_key` (or `x-license-key`)
   - `provider`, `model`
   - `instance`, `machine_id`
3. **scribe-api** (`chat.rs`):
   - Resolves `user_id` from license
   - Checks token limit
   - Routes model by plan (ModelRouter)
   - Builds OpenRouter model ID (e.g. `openai/gpt-4o-mini`)
   - Calls OpenRouter, streams SSE back
   - Records usage from stream (or estimates from content)
4. **Tauri**: Parses SSE, emits `chat_stream_chunk` and `chat_stream_complete` to main window
5. **Frontend**: Listens for events, appends chunks to response

---

## Usage Tracking

| Source | How Recorded |
|-------|--------------|
| **Scribe API** | Backend parses usage from OpenRouter stream; fallback: estimate from `content_chars/4` |
| **Direct (Exora, etc.)** | Frontend estimates tokens (`chars/4`), calls `POST /api/v1/usage/record` with `x-license-key` |

**Usage Dashboard** (`UsageDashboard.tsx`):
- Fetches `GET /api/v1/usage/{userId}` and `GET /api/v1/usage/{userId}/history`
- Requires `userId` from `POST /api/v1/auth/get-user` (license in body)
- Shows tokens used, limit, model breakdown, recent activity

---

## API Endpoints (scribe-api)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/api/v1/chat` | Chat with SSE streaming |
| POST | `/api/v1/audio` | Speech-to-text (Whisper) |
| POST | `/api/v1/models` | List OpenRouter models |
| POST | `/api/v1/auth/register` | Register user, get license |
| POST | `/api/v1/auth/get-user` | Get user_id from license |
| GET | `/api/v1/usage/:user_id` | Usage stats |
| GET | `/api/v1/usage/:user_id/history` | Recent messages |
| POST | `/api/v1/usage/record` | Record usage from client |
| WS | `/gateway` | Agent mode WebSocket |

---

## Configuration

### scribe-api (`.env`)

```env
DATABASE_URL=postgresql://...
API_ACCESS_KEY=your-key
OPENROUTER_API_KEY=sk-or-...
OPENAI_API_KEY=sk-...          # For Whisper
PORT=8083
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2
```

### Tauri (scribe/src-tauri)

- `APP_ENDPOINT` or `USE_LOCAL_BACKEND=1` → `http://127.0.0.1:8083`
- `API_ACCESS_KEY` → Bearer token for scribe-api

### Frontend (scribe/.env)

- `VITE_API_URL` → scribe-api base URL (default `http://localhost:8083`)
- `VITE_GHOST_GATEWAY_WS_URL` → WebSocket for agent mode

---

## Database Schema (PostgreSQL)

### Core Tables

- **users**: `plan`, `monthly_token_limit`, `tokens_used_this_month`, `monthly_reset_at`
- **licenses**: `license_key`, `user_id`, `status`, `tier`
- **messages**: `user_id`, `model`, `provider`, `prompt_tokens`, `completion_tokens`, `cost_usd`
- **monthly_usage**: `user_id`, `month`, `total_tokens`, `model_usage` (JSONB)
- **model_pricing**: `model`, `provider`, `input_cost_per_1m`, `output_cost_per_1m`

### Migrations

- `001_initial_schema.sql` – users, licenses, instances
- `002_usage_tracking.sql` – messages, monthly_usage, model_pricing, token limits

---

## Model Routing (Plan-Based)

When `user_id` is present, the ModelRouter:

1. Checks token limit; blocks if exceeded
2. If user requested a model, validates it against plan
3. Routes by task type (Chat, Code, Analysis) and plan

**Plan defaults** (examples):
- Free: `gpt-4o-mini`
- Starter: Chat `gpt-4o-mini`, Code `claude-3-haiku`
- Pro: Chat `gpt-4o`, Code `claude-3-5-sonnet`

**OpenRouter model ID**: When the routed model has no `/`, the code uses `ModelRouter::get_provider()` to build the correct ID (e.g. `gpt-4o-mini` → `openai/gpt-4o-mini`). It does **not** use the request’s provider header, which can be wrong when the model was routed.

---

## Key Fixes Applied

| Issue | Fix |
|-------|-----|
| **UI not updating** (chunks received, no response) | Use `app.emit_to("main", "chat_stream_chunk", ...)` instead of `app.emit()` so the main window receives events |
| **Usage not recorded** | Accept `license_key` header in chat.rs and usage.rs (Tauri sends `license_key`, API expected `x-license-key`) |
| **Usage not shown when stream has no usage** | Accumulate `content_chars` from stream; estimate tokens as `content_chars/4` when usage is missing |
| **Invalid model ID** (e.g. `nvidia/gpt-4o-mini`) | When model is routed, use `ModelRouter::get_provider(&model)` for OpenRouter ID instead of request provider |

---

## Running Locally

```bash
# 1. scribe-api
cd scribe/scribe-api
cp .env.example .env   # Set DATABASE_URL, OPENROUTER_API_KEY, etc.
cargo run

# 2. Tauri app
cd scribe
npm install
npm run tauri dev
```

Ensure scribe-api is on port 8083 and migrations have run.

---

## Admin Dashboard

Separate admin stack for analytics (admin-only):

- **admin-api** (Rust, port 8084): Login (username/password), JWT auth, analytics endpoints
- **admin-ui** (React, port 5174): Login page, dashboard with global stats, model breakdown, top users, recent messages

See `admin/README.md` for setup. Requires `DATABASE_URL`, `ADMIN_USERNAME`, `ADMIN_PASSWORD`, `ADMIN_SECRET`.

---

## Related Docs

- `scribe-api/SUMMARY.md` – Implementation summary
- `scribe-api/INTEGRATION.md` – API integration guide
- `scribe-api/LOCAL_SETUP.md` – Local setup
- `admin/README.md` – Admin dashboard setup
