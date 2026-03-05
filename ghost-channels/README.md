# Ghost Channels

Channel server for Ghost: Telegram and WhatsApp adapters that connect to scribe-api's WebSocket gateway. Inbound messages are dispatched to the gateway; replies are sent back over the same channel.

## Architecture

- **Channel server** (this package): Runs Telegram bot and WhatsApp client. Connects to scribe-api as a WebSocket client. Does not run the agent; it forwards to the gateway and delivers replies.
- **Session key**: One transcript per conversation, e.g. `telegram:${chatId}`, `whatsapp:${jid}`.
- **Flow**: Inbound message → build context (Body, From, To, SessionKey) → `chat.send` over WS → consume run.chunk / run.done / run.error → send reply via channel outbound.

## Config

- **File**: `~/.ghost/ghost.json` (or set `GHOST_CONFIG_PATH`).
- **Env**:
  - `GHOST_GATEWAY_URL` or `VITE_GHOST_GATEWAY_WS_URL` — WebSocket URL of scribe-api gateway (default `ws://127.0.0.1:8081/gateway`).
  - `GHOST_TELEGRAM_BOT_TOKEN` — Telegram bot token (enables Telegram).
  - `GHOST_WHATSAPP_AUTH_DIR` — Path to WhatsApp auth directory (enables WhatsApp; first run will print QR to pair).

Example `~/.ghost/ghost.json`:

```json
{
  "gateway": { "url": "ws://127.0.0.1:8081/gateway" },
  "channels": {
    "telegram": { "botToken": "YOUR_BOT_TOKEN" },
    "whatsapp": { "authDir": "~/.ghost/whatsapp-auth" }
  }
}
```

## Run

1. Start scribe-api (with gateway) so the WebSocket is available.
2. From this directory: `npm install && npm run build && npm start` (or `node dist/entry.js`).

For WhatsApp, the first run will print a QR code in the terminal; scan it with WhatsApp to pair.

## Requirements

- Node 18+
- scribe-api running with gateway enabled (e.g. `cargo run` from scribe-api).
