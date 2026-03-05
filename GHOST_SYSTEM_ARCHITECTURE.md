# Ghost System Architecture — Conceptual Overview

A high-level, presentation-friendly view of how Ghost works. No deep technicals—just the big picture.

---

## What is Ghost?

**Ghost** is a focused, single-app AI assistant system. You talk to it from a desktop app (Scribe) or from messaging channels (Telegram, WhatsApp). One agent, one control plane: observe the world, decide what to do, execute capabilities, and respond—with optional permission and input dialogs so you stay in control.

---

## System at a Glance

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           WHERE USERS TALK TO GHOST                          │
├─────────────────────────────────────────────────────────────────────────────┤
│  Scribe (Desktop)          │  Telegram / WhatsApp (ghost-channels)           │
│  Tauri + React app         │  Node adapters → forward to gateway            │
└──────────────┬──────────────┴────────────────────┬──────────────────────────┘
               │                                    │
               │  WebSocket / REST                   │  WebSocket (gateway)
               ▼                                    ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         CONTROL PLANE & AGENT BRAIN                           │
├──────────────────────────────────────────────────────────────────────────────┤
│  scribe-api (Rust)                                                            │
│  • REST API (chat, audio, auth, models)                                        │
│  • Gateway WebSocket at /gateway — same protocol as ghost-gateway             │
│  • Sessions, transcripts, agent runs                                          │
│                                                                               │
│  Optional: ghost-gateway (Node) — standalone gateway for agent mode only     │
│  (sessions, chat.send, chat.inject; no channels; port 18789)                  │
└──────────────────────────────────────────────────────────────────────────────┘
               │
               │  Where the agent actually runs
               ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│  Scribe desktop (Tauri) — AGENT RUNTIME                                       │
│  • Observe → Decide (Ollama planner) → Execute capability → Events            │
│  • Capabilities: filesystem read/write, process spawn                         │
│  • Permissions & input requests; execution tickets; belief state               │
└──────────────────────────────────────────────────────────────────────────────┘
```

**In short:** Users hit **Scribe** (desktop) or **channels** (Telegram/WhatsApp). Channels and optional **ghost-gateway** speak the same WebSocket protocol as **scribe-api**. The **agent loop** (observe → decide → execute) runs inside the **Scribe** desktop app; scribe-api coordinates sessions, transcripts, and gateway traffic and can forward agent runs to the desktop when a channel sends a message.

---

## Core Components

| Component | Role |
|-----------|------|
| **Scribe** | Desktop app (Tauri + React). The main UI and the **agent runtime**: runs the observe–decide–execute loop, calls Ollama for planning, executes capabilities (read/write files, run commands), and emits events. Can connect to scribe-api for chat/audio and to the gateway as “operator.” |
| **scribe-api** | Rust backend (Axum). REST for chat, audio (Whisper), auth, models (OpenRouter), plus a **WebSocket gateway** at `/gateway` for agent mode (connect, chat.history, chat.send, chat.inject, permission/input forwarding). Stores sessions and transcripts; does not run the agent. |
| **ghost-gateway** | Optional Node gateway (agent mode only). Same protocol as scribe-api’s gateway: sessions, chat.send, chat.inject. Used when you want a dedicated gateway (e.g. port 18789) without running the full scribe-api. |
| **ghost-channels** | Node service: Telegram bot and WhatsApp client. Connects to scribe-api (or ghost-gateway) as a WebSocket client. One transcript per conversation (e.g. `telegram:chatId`). Inbound message → `chat.send` over WS → consume run chunks → send reply back on the channel. |

---

## How a Request Flows

**From the desktop (Scribe)**  
User enters a goal in Scribe → app creates a run and starts the agent loop locally → loop observes state, asks the planner (Ollama) for the next decision, executes a capability (or asks for permission/input), appends events → UI shows run state and events; user can grant/deny permissions or answer input prompts.

**From Telegram or WhatsApp (ghost-channels)**  
User sends a message → channel server turns it into a `chat.send` over WebSocket to scribe-api (or ghost-gateway) → API appends the message to the session transcript and starts or triggers an agent run. If the agent runs on the desktop, scribe-api forwards the run to the connected Scribe client; when the agent needs permission or input, the API forwards those to the channel so the user can reply; replies are sent back to the desktop so the loop can continue. When the run produces a reply, it’s sent back over the channel.

**Shared idea**  
One session per conversation; one run per “turn.” The agent always runs in the same way (observe → decide → execute); the only difference is who initiated (desktop vs channel) and where permission/input dialogs are shown (desktop UI vs channel messages).

---

## The Agent Brain: Observe → Decide → Execute

The agent is a loop that never “knows” about files or terminals directly—only about **capabilities** (e.g. `filesystem.read`, `filesystem.write`, `process.spawn`).

1. **Observe**  
   Collect current state: run state, recent events, environment (e.g. working directory), belief state (working memory), and any recent “reflections” from past runs. No reasoning—just data for the next step.

2. **Decide**  
   A **planner** (Ollama LLM) gets the observation and a list of available capabilities. It returns one of: **invoke a capability** (with intent, inputs, expected outcome), **revise the plan**, **ask the user** a question, or **finish**. This is the only LLM call in the loop; everything else is deterministic.

3. **Execute**  
   If the decision is “invoke capability,” the system checks permission (and optionally asks the user), then runs the capability. Capabilities emit events (e.g. tool executed, step completed, artifact created). If the decision is “ask user,” the loop pauses until the user replies (desktop or via gateway/channel). Results feed back into the next observation.

So: **observe** (gather state) → **decide** (one LLM step) → **execute** (run a capability or ask user) → repeat until the planner says “finish” or the run fails/cancels.

---

## Capabilities and Safety

- **Capabilities** are the only way the agent can act. Each has a name, description, risk level, input schema, and optional permission/input requirements.
- **Execution tickets** track each tool/capability call (e.g. pending → permission → running → completed/failed). Restart-safe and auditable.
- **Permissions**: for sensitive actions the loop can pause and emit a permission request; the UI (or channel) shows it; the user grants or denies; the loop resumes with the result.
- **Input requests**: when the planner decides it needs more info (e.g. file path), it can emit an “ask_user” decision; the loop waits for the user’s reply before continuing.

So the agent never “does something dangerous” without going through a capability, and capabilities can require explicit permission or input.

---

## Data and State

- **Sessions**  
  One logical conversation per user or channel thread (e.g. `telegram:123`, `whatsapp:jid`). Transcripts are stored (e.g. JSONL under `~/.ghost/sessions/` by ghost-gateway or scribe-api).

- **Runs and events**  
  Each agent “turn” is a run with a goal. All steps are recorded as **events** (e.g. plan created, decision made, capability invoked, step evaluated, permission requested). Run state is derived by replaying events (and optionally persisted). This gives a clear audit trail and lets the UI show what the agent did.

- **Belief state**  
  A lightweight “working memory” derived from events: what the agent currently believes (e.g. files read, commands run). Used in the next observation so the planner has context.

---

## Where Moltbot Fits (Optional)

The repo also contains **moltbot** (Clawdbot), a separate, full-featured multi-channel assistant (WhatsApp, Telegram, Slack, Discord, etc.) with its own gateway, Pi agent runtime, and many extensions. **Ghost** is a smaller, focused system: one desktop app (Scribe), one API (scribe-api) with an embedded gateway, and optional channel adapters (ghost-channels). Ghost reuses protocol ideas from Moltbot (e.g. gateway methods like `chat.send`) but does not depend on Moltbot; the agent loop and capabilities are implemented inside Scribe (Tauri/Rust). Think of Ghost as “one agent, one app, optional channels”; Moltbot as “one gateway, many channels and skills.”

---

## Summary for a Presentation

- **Ghost** = Scribe (desktop + agent) + scribe-api (REST + gateway) + optional ghost-gateway + ghost-channels (Telegram/WhatsApp).
- **Users** talk via the Scribe UI or via Telegram/WhatsApp; channels and gateway share the same WebSocket protocol.
- **Agent** = a loop: observe state → planner (Ollama) decides one step → execute a capability or ask user → record events and repeat.
- **Safety** = capabilities only, execution tickets, permission and input dialogs, event-sourced runs.
- **State** = sessions and transcripts for conversation; runs and events for the agent’s actions and audit.

This is the conceptual architecture of Ghost—enough to explain the system in a presentation without diving into implementation details.
