# Ghost Agent Architecture

This document describes how the Ghost **Agent Mode** is structured end-to-end,
covering the data model, event flow, tools, and UI surfaces.

## High-level overview

The agent is an **event-sourced system**. Runs and all state changes are stored
as immutable events in SQLite. The UI reconstructs state by replaying events
into a `RunState` projection. The agent loop emits events for every action,
tool execution, and status change.

Core layers:
- **Rust/Tauri backend**: run manager, agent loop, tools, event store.
- **SQLite**: `runs` and `run_events` tables.
- **Frontend**: `AgentView` renders the run state and events.

## Key modules and responsibilities

### Agent loop (decision + execution)
`src-tauri/src/agent/loop/run_loop.rs`
- Continuously observes the current run state and decides the next action.
- Valid actions: `fs_read`, `fs_write`, `ask_permission`, `ask_user`, `finish`.
- Emits projection events for status, steps, permissions, and messages.
- Maintains cancellation and restart safety.

### Run manager
`src-tauri/src/agent/loop/run_loop.rs` (`RunManager`)
- Starts runs in the background and tracks active runs.
- Handles permission decision channels.
- Resumes non-terminal runs on app start.

### Event store
`src-tauri/src/agent/run/store.rs`
- Writes `runs` and `run_events` in SQLite via `sqlx`.
- Emits the `run_event` Tauri event to the frontend.
- Loads events and rebuilds run state by replaying them.

### Run state projection
`src-tauri/src/agent/state/run_state.rs`
- `RunState` is derived from events (not authoritative).
- Event reducer applies events in order to build state for UI.

### Tools
`src-tauri/src/agent/tools/*`
- Each tool emits a consistent sequence:
  - `TOOL_EXECUTED` (fact)
  - domain-specific events (ex: `FILE_WRITTEN`)
  - `STEP_COMPLETED` (projection)
  - `ARTIFACT_CREATED` (projection)
- Example: `fs_write` emits an artifact with `location` and summary.

### Permissions
`src-tauri/src/agent/permissions/*` and `run_loop`
- Tools can request permissions via `ask_permission`.
- UI replies through `reply_permission`, which updates status.

### UI
`src/components/agent/AgentView.tsx`
- Shows run status, artifacts, messages, steps, and events.
- Reloads state when `run_event` is emitted.

## Data model and storage

SQLite tables:
- `runs`: run metadata (`id`, `goal`, `status`, etc.)
- `run_events`: append-only log (`run_id`, `event_type`, `payload`, `created_at`)

The UI never mutates state directly; it requests the current projection via
`get_run_state` which replays events.

## Event flow (simplified)

1. **Create run**
   - `create_run` inserts row into `runs`
   - Emits `run.created`

2. **Start run**
   - `RunManager.start_run` emits `run.status_changed: running`
   - `run_loop` begins decision cycle

3. **Decision**
   - `decide_next` produces JSON action
   - `run_loop` executes tool or asks permission/user

4. **Tool execution**
   - Tool emits `TOOL_EXECUTED` + `STEP_COMPLETED`
   - Tool emits `ARTIFACT_CREATED` when applicable

5. **Completion**
   - `finish` emits `run.status_changed: completed`

## UI surfaces and toggles

The app has two UI modes:
- **Chat mode**: classic input UI.
- **Agent mode**: AgentView with run state and event timeline.

Mode is persisted in local storage and can be toggled from:
- Settings (mode selector)
- Chat UI "Switch Mode" button
- Agent UI "Switch Mode" button

## File creation and path picking

`fs_write` behavior:
- If a path is missing or relative, a **Save File** dialog is opened.
- The selected path is used for the write, and the agent emits an assistant
  message with the final file location.

## Failure handling

Typical failure scenarios:
- Tool errors → `STEP_FAILED` event
- Permission denied → loop replans or waits
- Cancellation → `run.status_changed: cancelled`

The run projection will always reflect the last known status.

## Relevant files (quick links)

- `src-tauri/src/agent/loop/run_loop.rs`
- `src-tauri/src/agent/run/store.rs`
- `src-tauri/src/agent/state/run_state.rs`
- `src-tauri/src/agent/tools/fs_write.rs`
- `src/components/agent/AgentView.tsx`
- `src/App.tsx`
- `src/lib/storage/customizable.storage.ts`
