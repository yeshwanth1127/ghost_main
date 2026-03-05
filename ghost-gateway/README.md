# Ghost Gateway

Ghost gateway server for **agent mode** only. This is **our own implementation**: code is ported from the Moltbot codebase into Ghost; we do **not** wrap or depend on Moltbot.

- **Config:** `~/.ghost/ghost.json`
- **State:** `~/.ghost/` (sessions, transcripts)
- **Port:** 18789 (default)

## Usage

```bash
npm install
npm run build
npm run gateway
# or: node dist/entry.js --port 18789
```

## Protocol

- First WebSocket frame must be a **connect** request: `{ type: "req", id, method: "connect", params: { minProtocol, maxProtocol, client: { id, version, platform } } }`.
- Server responds with **hello-ok**: `{ type: "res", id, ok: true, payload: { type: "hello-ok", protocol, server, snapshot } }`.
- After that, client can send requests and receive responses.

## Methods (agent mode)

- **chat.history** – `params: { sessionKey, limit? }` → `{ sessionKey, messages }`. Transcript stored as JSONL under `~/.ghost/sessions/`.
- **chat.send** – `params: { sessionKey, message, idempotencyKey? }` → `{ runId, status: "started" }`. Appends user message to transcript; agent run is a separate step.
- **chat.inject** – `params: { sessionKey, message, label? }` → `{ ok, messageId }`. Appends assistant message without running the agent.

## Scope

Agent mode only. Chat mode is unchanged and not touched.
