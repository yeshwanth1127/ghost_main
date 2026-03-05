---
name: Ghost vs Moltbot Chatting Layer
overview: Port Moltbot's gateway, Pi agent, and channel architecture (WhatsApp, Telegram, Discord) into Ghost for agent mode only; leave chat mode completely untouched.
todos:
  - id: ghost-gateway-pkg
    content: Create ghost-gateway package and port gateway server (connect, WS runtime, protocol)
    status: completed
  - id: chat-session-transcript
    content: Port session store and transcript (JSONL) + chat.history, chat.send, chat.inject
    status: completed
  - id: agent-method-pi
    content: Port agent gateway method and Pi agent (pi-embedded-runner) for agent mode
    status: pending
  - id: channel-architecture
    content: Port same code/architecture for WhatsApp, Telegram, Discord (inbound, outbound, gateway adapter)
    status: pending
  - id: ghost-ui-agent-mode
    content: Wire agent-mode UI to gateway (WebSocket client); do not touch chat mode
    status: pending
  - id: gateway-startup
    content: Gateway startup (CLI or Tauri sidecar) and config (~/.ghost/ghost.json)
    status: pending
isProject: false
---

# Ghost Agent Mode = Moltbot Ideology (Chat Mode Untouched)

## Scope: Agent Mode Only

- **Chat mode:** Do **not** touch. Existing chat mode (scribe-api, [useCompletion](scribe/scribe/src/hooks/useCompletion.ts), HTTP/SSE to OpenRouter, local conversations) remains unchanged.
- **Agent mode only:** All implementation work is for **agent mode**. Gateway, Pi agent, chat protocol (chat.history, chat.send, chat.inject, agent), and channel integrations (WhatsApp, Telegram, Discord) are for agent-mode chatting only.
- **Pi agent:** Use Moltbot's **Pi agent** ([pi-embedded-runner](moltbot/src/agents/pi-embedded-runner/)) for agent mode implementation—same code and architecture.
- **Channels:** Use the **same code and architecture** as Moltbot for how WhatsApp, Telegram, and Discord implement chatting: gateway connection, inbound message → dispatch → agent run, outbound reply. Port that architecture into Ghost for agent mode.

---

## What Matches Moltbot (and What to Port)

Your [GHOST_ARCHITECTURE.md](GHOST_ARCHITECTURE.md) already describes the same system as Moltbot. The **implementation** lives in moltbot; Ghost currently has no separate gateway—only the Ghost UI (scribe) talking to scribe-api over HTTP for chat.

**Port into Ghost (from moltbot):**

- **Gateway server:** WebSocket server, first frame `connect`, then request/response + events. Source: [moltbot/src/gateway/server.impl.ts](moltbot/src/gateway/server.impl.ts), [server-ws-runtime.ts](moltbot/src/gateway/server-ws-runtime.ts), [server.ts](moltbot/src/gateway/server.ts).
- **Protocol:** Typed methods, dedupe, auth. Source: [moltbot/src/gateway/protocol/](moltbot/src/gateway/protocol/), [server-methods.ts](moltbot/src/gateway/server-methods.ts).
- **Connect:** Handshake, hello-ok, presence/health snapshot. Source: [moltbot/src/gateway/server-methods/connect.ts](moltbot/src/gateway/server-methods/connect.ts).
- **Chat:** `chat.history`, `chat.send`, `chat.inject`. Source: [moltbot/src/gateway/server-methods/chat.ts](moltbot/src/gateway/server-methods/chat.ts).
- **Sessions + transcript:** Session key → JSONL transcript; session store. Source: [moltbot/src/config/sessions/](moltbot/src/config/sessions/), [gateway/session-utils.ts](moltbot/src/gateway/session-utils.ts), chat.ts (resolveTranscriptPath, appendAssistantTranscriptMessage, readSessionMessages).
- **Agent method:** `agent` request → run Pi (or minimal runner), stream via `event:agent`. Source: [moltbot/src/gateway/server-methods/agent.ts](moltbot/src/gateway/server-methods/agent.ts), [moltbot/src/commands/agent.ts](moltbot/src/commands/agent.ts), [moltbot/src/agents/pi-embedded-runner/](moltbot/src/agents/pi-embedded-runner/).
- **Chat event wiring:** When agent streams, broadcast to clients. Source: [moltbot/src/gateway/server-chat.ts](moltbot/src/gateway/server-chat.ts).

**Agent-mode UI only:** In [scribe/scribe](scribe/scribe), **agent mode** (not chat mode) talks to the Ghost gateway: add a WebSocket client for agent mode that uses `chat.history`, `chat.send`, and `event:agent`. **Chat mode** (existing completion, scribe-api, useCompletion) is **not modified**.

---

## Execution Engine: Moltbot vs Ghost

### Moltbot (Pi embedded runner)

- **Where:** [moltbot/src/agents/pi-embedded-runner/run/attempt.ts](moltbot/src/agents/pi-embedded-runner/run/attempt.ts); uses `@mariozechner/pi-coding-agent`, `@mariozechner/pi-ai`.
- **Model:** One run = one continuous loop: load transcript (JSONL) → build system prompt (AGENTS.md, SOUL, TOOLS) → create tools (e.g. `createMoltbotCodingTools`) → call LLM (`streamSimple`) → on tool calls, execute tools → append to transcript → repeat until done. **Transcript is the source of truth**; no event log.
- **Pros:** Simple, proven for chat + coding agent, multi-channel (WhatsApp, Telegram, WebChat). Single process (Node), easy to reason about.
- **Cons:** No first-class audit trail (only transcript); restart mid-run means replay from transcript (no fine-grained "resume after last side-effect"); no built-in permission/approval flow in the loop.

### Ghost (event-sourced agent engine)

- **Where:** [.cursor/plans/ghost_agent_engine_implementation_db149b78.plan.md](.cursor/plans/ghost_agent_engine_implementation_db149b78.plan.md); implemented in scribe `src-tauri/src/agent/`.
- **Model:** Event-sourced: `runs` + `run_events`. RunState is a **projection** from events. Loop: observe_state → decide_next (LLM) → execute tool (fs_read, fs_write) → emit events. Restart-safe: replay events, find last completed side-effect, resume. Permissions and artifacts are part of the model.
- **Pros:** Audit trail, restart safety, permission flow, good for desktop "agent mode" with file ops and safety.
- **Cons:** More complex; not designed for multi-channel gateway; runs in Tauri (Rust), separate from a Node gateway.

### Which is better?

- **For the gateway + multi-channel chat (Moltbot-style):** **Moltbot's Pi runner is better.** It's built for that: one transcript per session, one LLM+tools loop, streaming back to the gateway and to any channel. Use it (or a port) as the execution engine for gateway-triggered agent runs.
- **For the desktop Ghost app "agent mode" (file ops, permissions, audit):** **Ghost's event-sourced engine is better.** Keep it for the Tauri-side agent runs; it doesn't replace the gateway's need for a simple, stream-friendly runner.

**Recommendation:** For **agent mode**, use **Pi agent** (moltbot's pi-embedded-runner)—same code and architecture. Do not replace it with Ghost's event-sourced engine for gateway/channel flows; the event-sourced engine (if used at all) stays separate for desktop-only use. All gateway-triggered agent runs use Pi agent.

---

## Implementation Plan (Basics First)

### 1. Ghost gateway package

- **Create** a new Node/TS package, e.g. `ghost_main/ghost-gateway/` (or `gateway/`), that will contain the gateway server.
- **Port** from moltbot (copy/adapt, minimal renames to "ghost" where appropriate):
- Gateway entry and config: config load (`~/.ghost/ghost.json`), resolve port (e.g. 18789), bind.
- WebSocket server: [server.impl.ts](moltbot/src/gateway/server.impl.ts) (startGatewayServer), [server-ws-runtime.ts](moltbot/src/gateway/server-ws-runtime.ts), [server.ts](moltbot/src/gateway/server.ts).
- Protocol: connect handshake, request/response, events; [protocol/](moltbot/src/gateway/protocol/), [server-methods-list.ts](moltbot/src/gateway/server-methods-list.ts).
- **Connect** handler: [server-methods/connect.ts](moltbot/src/gateway/server-methods/connect.ts) (hello-ok, presence/health snapshot).

Dependencies: either add moltbot as a dependency and call into it (fastest), or copy the needed gateway files into `ghost-gateway` and replace internal imports with ghost-specific ones (cleaner long-term).

### 2. Chat and sessions

- **Port** into ghost-gateway:
- Session store and transcript path resolution: [session-utils.ts](moltbot/src/gateway/session-utils.ts), session config from [config/sessions](moltbot/src/config/sessions/).
- **chat.history:** read messages from session transcript (cap/cursor if you want; moltbot has getMaxChatHistoryMessagesBytes, capArrayByJsonBytes).
- **chat.send:** idempotency, append user message to transcript, optionally start agent run, return runId/status.
- **chat.inject:** append assistant message to transcript, broadcast to clients.
- **Transcript format:** Same as Moltbot (JSONL: session header line then one JSON object per message). Location: e.g. `~/.ghost/sessions/` or configurable store path.

### 3. Agent method and streaming

- **Port** into ghost-gateway:
- **agent** method: [server-methods/agent.ts](moltbot/src/gateway/server-methods/agent.ts) (validate params, resolve session, call agent command).
- Agent execution: Use **Pi agent**—[commands/agent.ts](moltbot/src/commands/agent.ts) and [pi-embedded-runner/run/attempt.ts](moltbot/src/agents/pi-embedded-runner/run/attempt.ts). Port or depend on moltbot's same code and architecture (no minimal runner; use Pi agent for agent mode).
- **event:agent** streaming: [server-chat.ts](moltbot/src/gateway/server-chat.ts) (createAgentEventHandler), broadcast chunks and final status to subscribed clients.

This gives the gateway + Pi agent for agent mode at the protocol level.

### 4. Channel architecture (WhatsApp, Telegram, Discord)

- **Use the same code and architecture** as Moltbot for how WhatsApp, Telegram, and Discord implement chatting. Port into ghost-gateway (or a ghost-channels package):
- **Channel plugins:** [moltbot/src/channels/plugins/](moltbot/src/channels/plugins/): config adapters, gateway adapter (connect to gateway WS), types ([types.adapters.ts](moltbot/src/channels/plugins/types.adapters.ts), [types.plugin.ts](moltbot/src/channels/plugins/types.plugin.ts)).
- **WhatsApp (web):** [moltbot/src/web/](moltbot/src/web/) (inbound, outbound, session, auto-reply), [channel-web.ts](moltbot/src/channel-web.ts). Inbound message → build context → dispatch; outbound via gateway send.
- **Telegram:** [moltbot/src/telegram/](moltbot/src/telegram/): bot, [bot-message-dispatch.ts](moltbot/src/telegram/bot-message-dispatch.ts), send, outbound adapter. Same flow: receive → dispatchInboundMessage → agent; reply via outbound.
- **Discord:** [moltbot/src/discord/](moltbot/src/discord/): [monitor](moltbot/src/discord/monitor/), [message-handler](moltbot/src/discord/monitor/message-handler/), send. Receive → dispatch → agent; reply via send.
- **Auto-reply / dispatch:** [moltbot/src/auto-reply/dispatch.ts](moltbot/src/auto-reply/dispatch.ts) (dispatchInboundMessage), [reply-dispatcher](moltbot/src/auto-reply/reply/), [dispatch-from-config](moltbot/src/auto-reply/reply/dispatch-from-config.js). Channels call this; it resolves session, runs agent (via gateway), sends reply through dispatcher.
- **Channel manager:** [moltbot/src/gateway/server-channels.ts](moltbot/src/gateway/server-channels.ts) (createChannelManager, startChannels, startChannel). Gateway starts channel runtimes; each channel plugin has gateway.startAccount (or equivalent) that connects to platform and registers inbound handler that calls dispatchInboundMessage.

Port the same flow: channel receives message → build MsgContext → dispatchInboundMessage(cfg, dispatcher) → agent run (Pi) → dispatcher sends reply back through channel outbound. Do not touch chat mode; this is agent-mode chatting only.

### 5. Ghost UI: agent mode only

- **Do not touch chat mode.** Chat mode (useCompletion, scribe-api, existing completion UI) stays as-is.
- **Agent mode only:** When the user is in **agent mode**, the agent-mode UI connects to the Ghost gateway via WebSocket (e.g. `lib/gateway-client.ts` or hook for agent mode):
- Connect to `ws://127.0.0.1:18789`, first frame `connect`.
- `chat.history(sessionKey)` to load messages; `chat.send(sessionKey, text, idempotencyKey)` to send; listen for `event:agent` for streaming.
- Session key for desktop agent UI can be fixed (e.g. `webchat:default`). No changes to chat mode or chat-mode UI.

### 6. Gateway startup

- **Option A:** Standalone CLI (e.g. `ghost gateway`) that runs `startGatewayServer(port)`. User runs it in a terminal or as a background service.
- **Option B:** Tauri sidecar: Ghost app spawns the gateway process (e.g. `ghost-gateway` binary or `node ghost-gateway/dist/entry.js`) when "Gateway mode" is on; kill on app exit. Easiest for "single app" UX; same ideology as Moltbot.

Config: `~/.ghost/ghost.json` (port, bind, auth, session store path). Port from moltbot's config schema or start with a minimal one.

---

## Summary

| Item | Action |
|------|--------|
| Scope | **Agent mode only.** Do not touch chat mode (useCompletion, scribe-api, existing chat UI). |
| Execution engine | **Pi agent** for agent mode—same code and architecture as Moltbot ([pi-embedded-runner](moltbot/src/agents/pi-embedded-runner/), [commands/agent](moltbot/src/commands/agent.ts)). |
| Gateway + chat | Port connect, chat.*, sessions/transcript, agent method + event:agent from moltbot into ghost-gateway (for agent mode). |
| Channels | Port **same code and architecture** as Moltbot for WhatsApp, Telegram, Discord: channel plugins, inbound → dispatchInboundMessage → Pi agent, outbound; [channels/plugins](moltbot/src/channels/plugins/), [web](moltbot/src/web/), [telegram](moltbot/src/telegram/), [discord](moltbot/src/discord/), [server-channels](moltbot/src/gateway/server-channels.ts). |
| Ghost UI | Only **agent mode** talks to the gateway (WebSocket client). Chat mode is untouched. |
| Start with | Gateway package, connect, chat.*, session/transcript, Pi agent + streaming; then channel architecture (WhatsApp, Telegram, Discord); then agent-mode UI client + gateway startup. |