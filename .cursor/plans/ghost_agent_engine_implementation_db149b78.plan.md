---
name: Ghost Agent Engine Implementation
overview: Implement a complete event-sourced agent engine with persistent run state, isolated from existing assistant logic, plus a mode selection UI for chat vs agent modes.
todos:
  - id: phase0-structure
    content: "Phase 0: Create agent module structure and UI mode selection component"
    status: completed
  - id: phase1-db
    content: "Phase 1.1: Create database migration for runs and run_events tables"
    status: completed
  - id: phase1-events
    content: "Phase 1.2-1.3: Implement event model and canonical event types"
    status: completed
  - id: phase1-state
    content: "Phase 1.4: Implement RunState model and pure reducer function"
    status: completed
  - id: phase1-store
    content: "Phase 1.5: Implement run store with create_run, append_event, load_run_state"
    status: completed
  - id: phase1-commands
    content: "Phase 1.6-1.7: Add Tauri commands and event bridge, test Phase 1 exit criteria"
    status: completed
  - id: phase2-loop
    content: "Phase 2.1-2.2: Implement orchestrator loop and observation model"
    status: completed
  - id: phase2-decision
    content: "Phase 2.3: Implement LLM decision model with JSON schema validation"
    status: completed
  - id: phase2-tools
    content: "Phase 2.4: Implement fs_read and fs_write tools with event emission"
    status: completed
  - id: phase2-commands
    content: "Phase 2.5: Add start_run, cancel_run, reply_permission commands, test Phase 2 exit criteria"
    status: completed
  - id: phase3-permissions
    content: "Phase 3: Implement permission model and blocking flow in loop"
    status: completed
  - id: phase4-artifacts
    content: "Phase 4: Implement artifact model, extraction, summarization, and retrieval in observations"
    status: completed
  - id: phase5-plans
    content: "Phase 5: Handle plan artifacts as derived projections (minimal implementation)"
    status: completed
  - id: phase6-recovery
    content: "Phase 6: Implement recovery strategies (retry, alternate tools, graceful failure)"
    status: completed
  - id: phase7-headless
    content: "Phase 7: Implement headless execution, background task management, and UI reconnect"
    status: completed
isProject: false
---

# Ghost Agent Engine — Implementation Plan

## Non-Negotiable Principles

- The agent loop owns time and execution.
- UI never mutates run state directly.
- All state changes are event-sourced.
- LLM is stateless and advisory only.
- Runs must survive UI disconnects and app restarts.
- Everything observable must be reconstructable from events (+ snapshots).

### Event Ontology Rule (CRITICAL)

**Events must describe what happened, not what it means.**

- ✅ **Facts**: `tool.executed` with output, `file.written`, `permission.requested`
- ⚠️ **Projections**: `step.completed`, `run.completed` (these are interpretations, not facts)

**Why this matters:**

- If events encode interpretations, you lock yourself into today's logic forever.
- Events are the authoritative record. Projections can change; events cannot.
- Document explicitly: Step semantics (`step.started`, `step.completed`) are **projections**, not ontology.

**If Cursor treats projection events as "truth," the implementation is wrong.**

### RunState Projection Rule (CRITICAL)

**RunState is a derived, lossy projection. Events are the only authoritative record.**

- RunState reconstructs: messages, steps, permissions, artifacts
- This is a **view**, not reality
- Future projections may exist (timeline view, audit view, debug view)
- If Cursor assumes RunState is "the model" and mutates it directly, the implementation is wrong.

### Restart Safety Rule (CRITICAL)

**The agent loop must be restart-safe and idempotent with respect to events.**

On app restart, the loop must:

1. Replay all events for the run
2. Detect the last completed side-effect (via events)
3. Continue from after the last confirmed side-effect
4. Never duplicate side effects

**Concretely:**

- If a run was mid-tool: Detect incomplete tool execution, do not restart tool
- If a run was waiting for permission: Resume waiting state
- If a run was partially through a file write: Detect partial write, handle appropriately

**If restart logic duplicates side effects, the implementation is wrong.**

If any implementation violates these, it is wrong.

---

## Phase 0 — Ground Rules & UI Mode Selection

### 0.1 Create Isolated Agent Module Structure

Create new folder structure in `src-tauri/src/agent/`:

```
src-tauri/src/agent/
├── mod.rs
├── run/
│   ├── mod.rs
│   └── store.rs
├── events/
│   ├── mod.rs
│   └── event.rs
├── state/
│   ├── mod.rs
│   └── run_state.rs
├── loop/
│   ├── mod.rs
│   └── run_loop.rs
├── tools/
│   ├── mod.rs
│   ├── fs_read.rs
│   └── fs_write.rs
├── memory/
│   ├── mod.rs
│   └── artifacts.rs
└── permissions/
    ├── mod.rs
    └── permission.rs
```

**Files to create:**

- `src-tauri/src/agent/mod.rs` - Module declaration
- All subdirectories and their `mod.rs` files

### 0.2 UI Mode Selection Component

**File:** `src/components/mode/ModeSelector.tsx`

Create a component that shows on app load with two options:

- **Chat Mode**: Shows existing `Completion` component
- **Agent Mode**: Placeholder for agent UI (to be designed in later phases)

**Integration point:** Modify `src/App.tsx` to:

1. Add state: `const [appMode, setAppMode] = useState<'chat' | 'agent' | null>(null)`
2. Show `ModeSelector` when `appMode === null`
3. Show `Completion` when `appMode === 'chat'`
4. Show agent UI when `appMode === 'agent'`

**Files to modify:**

- `src/App.tsx` - Add mode selection logic
- `src/components/mode/ModeSelector.tsx` - New component

---

## Phase 1 — Run + Event Log (NO AGENT LOGIC YET)

### 1.1 Database Schema Migration

**File:** `src-tauri/src/db/migrations/agent-runs.sql`

Create migration file with:

```sql
-- runs table
CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  goal TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- run_events table (append-only)
CREATE TABLE IF NOT EXISTS run_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (run_id) REFERENCES runs(id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_run_events_run_id ON run_events(run_id, id ASC);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs(status);
CREATE INDEX IF NOT EXISTS idx_runs_created_at ON runs(created_at DESC);
```

**Files to modify:**

- `src-tauri/src/db/main.rs` - Add new migration (version 4)
- `src-tauri/src/db/migrations/agent-runs.sql` - New file

### 1.2 Event Model

**File:** `src-tauri/src/agent/events/event.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    pub id: i64,
    pub run_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

**Files to create:**

- `src-tauri/src/agent/events/mod.rs`
- `src-tauri/src/agent/events/event.rs`

### 1.3 Canonical Event Types

**File:** `src-tauri/src/agent/events/mod.rs`

Define constants for event type strings.

**Fact Events (Ontology - What Happened):**

- `RUN_CREATED` - Run was created
- `TOOL_EXECUTED` - Tool was executed with specific output
- `FILE_WRITTEN` - File was written (with path and content hash)
- `FILE_READ` - File was read (with path and content hash)
- `PERMISSION_REQUESTED` - Permission was requested (with scope and reason)
- `PERMISSION_DECISION` - Permission decision was made (granted/denied)
- `MESSAGE_APPENDED` - Message was appended to run

**Projection Events (Interpretations - What It Means):**

- `RUN_STATUS_CHANGED` - Run status changed (derived from other events)
- `STEP_STARTED` - Step started (projection of tool execution)
- `STEP_COMPLETED` - Step completed (projection of successful tool execution)
- `STEP_FAILED` - Step failed (projection of failed tool execution)
- `ARTIFACT_CREATED` - Artifact created (projection of tool output)

**Documentation requirement:**

- Add explicit comments in code: "PROJECTION: This event type represents an interpretation, not a fact."
- Reducer must handle both fact and projection events, but fact events are authoritative.

### 1.4 RunState Model & Reducer

**File:** `src-tauri/src/agent/state/run_state.rs`

Define:

- `RunStatus` enum: `Pending`, `Running`, `WaitingPermission`, `Completed`, `Failed`, `Cancelled`
- `RunState` struct with fields: `id`, `goal`, `status`, `messages`, `steps`, `permissions`, `artifacts`
- `Message`, `Step`, `PermissionRequest`, `Artifact` structs
- `apply_event(state: &mut RunState, event: &RunEvent)` - Pure reducer function

**CRITICAL DOCUMENTATION:**

Add at the top of `run_state.rs`:

```rust
// ⚠️ IMPORTANT: RunState is a DERIVED PROJECTION, not authoritative state.
// 
// - Events in the database are the ONLY source of truth
// - RunState is reconstructed by replaying events
// - This is a VIEW for convenience, not the model
// - Future projections may exist (timeline, audit, debug views)
// - NEVER mutate RunState directly - always emit events
```

**Rules:**

- Reducer must be pure (no side effects, no DB calls)
- Each event type has explicit handling
- Unknown event types are ignored (no panic)
- Reducer builds a projection - it does not define reality
- Fact events are authoritative; projection events are derived

**Files to create:**

- `src-tauri/src/agent/state/mod.rs`
- `src-tauri/src/agent/state/run_state.rs`

### 1.5 Run Store (Database Operations)

**File:** `src-tauri/src/agent/run/store.rs`

Implement:

- `create_run(goal: String) -> Result<String, String>` - Creates run, emits `run.created`
- `append_event(run_id: &str, event_type: &str, payload: serde_json::Value) -> Result<RunEvent, String>`
- `load_run_events(run_id: &str) -> Result<Vec<RunEvent>, String>` - Ordered by `id ASC`
- `load_run_state(run_id: &str) -> Result<RunState, String>` - Replays all events

**Files to create:**

- `src-tauri/src/agent/run/mod.rs`
- `src-tauri/src/agent/run/store.rs`

### 1.6 Tauri Commands for Phase 1

**File:** `src-tauri/src/agent/mod.rs`

Add commands:

- `create_run(goal: String) -> Result<String, String>`
- `get_run_state(run_id: String) -> Result<RunState, String>`
- `get_run_events(run_id: String) -> Result<Vec<RunEvent>, String>`

**Files to modify:**

- `src-tauri/src/lib.rs` - Register agent module and commands

### 1.7 Tauri Event Bridge

**File:** `src-tauri/src/agent/run/store.rs`

In `append_event`, after database insert, emit Tauri event:

```rust
app.emit("run_event", &event)?;
```

**Files to modify:**

- `src-tauri/src/agent/run/store.rs` - Add AppHandle parameter to append_event

### 1.8 Exit Criteria for Phase 1

**Test checklist:**

- [ ] Create a run via command
- [ ] Manually append events via command
- [ ] Kill app
- [ ] Restart app
- [ ] Replay events
- [ ] Verify state is identical

**No loop yet.** If loop code appears, reject it.

---

## Phase 2 — Minimal Agent Loop

### 2.1 Orchestrator

**File:** `src-tauri/src/agent/loop/run_loop.rs`

Implement `run_loop(run_id: String, cancel: CancellationToken)`:

```rust
while run not terminal {
    let observation = observe_state(run_id).await?;
    let decision = decide_next(observation).await?;
    
    if decision.action == "ask_permission" {
        emit permission.asked
        wait for permission.replied
        continue
    }
    
    if decision.action == "finish" {
        emit run.status_changed -> Completed
        break
    }
    
    execute_tool(decision).await?;
    emit step events
}
```

**Files to create:**

- `src-tauri/src/agent/loop/mod.rs`
- `src-tauri/src/agent/loop/run_loop.rs`

### 2.2 Observation Model

**File:** `src-tauri/src/agent/loop/run_loop.rs`

```rust
pub struct Observation {
    pub run_state: RunState,
    pub recent_events: Vec<RunEvent>,
    pub environment: EnvironmentSnapshot,
    pub last_decision_event_id: Option<i64>, // Track last decision point
}
```

`observe_state()`:

1. Load `RunState` via replay
2. Load events since last decision (deterministic by event ID, NOT by count)
3. Collect environment snapshot (current directory, etc.)

**CRITICAL: recent_events Determinism Rule**

- `recent_events` must be defined by event IDs since last decision, NOT by count
- Store `last_decision_event_id` in loop state (or derive from events)
- Query: `SELECT * FROM run_events WHERE run_id = ? AND id > ? ORDER BY id ASC`
- This ensures:
  - Deterministic reasoning (same events → same LLM input)
  - Reproducible runs
  - Debuggable failures

**If recent_events uses "last N" or count-based selection, the implementation is wrong.**

### 2.3 Decision Model (LLM Call)

**File:** `src-tauri/src/agent/loop/run_loop.rs`

`decide_next(observation: Observation) -> Result<Decision, String>`:

- Input: Serialized `Observation` to JSON
- LLM call via existing `api::chat_stream` infrastructure
- Output schema validation:
```json
{
  "action": "fs_read | fs_write | ask_permission | ask_user | finish",
  "args": {...},
  "confidence": 0.0-1.0,
  "reason": "short explanation"
}
```


**Rules:**

- Must validate JSON schema
- Invalid output = emit `step.failed` + retry (max 3 times)
- Use existing API infrastructure from `src-tauri/src/api.rs`

### 2.4 Tool Execution (2 Tools Only)

**File:** `src-tauri/src/agent/tools/fs_read.rs`

Implement `fs_read(path: String, scope: CapabilityScope) -> Result<Artifact, String>`:

- Validate capability scope
- Emit **fact event**: `TOOL_EXECUTED` with tool name, args, output
- Emit **fact event**: `FILE_READ` with path and content hash
- Read file
- Emit **projection event**: `STEP_COMPLETED` (derived from successful tool execution)
- Emit **projection event**: `ARTIFACT_CREATED` (derived from tool output)
- Return artifact

**File:** `src-tauri/src/agent/tools/fs_write.rs`

Implement `fs_write(path: String, content: String, scope: CapabilityScope) -> Result<Artifact, String>`:

- Validate capability scope
- Emit **fact event**: `TOOL_EXECUTED` with tool name, args, output
- Emit **fact event**: `FILE_WRITTEN` with path and content hash
- Write file (atomic if possible, or use transaction markers)
- Emit **projection event**: `STEP_COMPLETED` (derived from successful tool execution)
- Emit **projection event**: `ARTIFACT_CREATED` (derived from tool output)
- Return artifact

**Event Ordering Rule:**

- Fact events (`TOOL_EXECUTED`, `FILE_WRITTEN`) must be emitted BEFORE side effects
- This enables restart safety: if app crashes, we can detect incomplete operations
- Projection events can be emitted after side effects complete

**Files to create:**

- `src-tauri/src/agent/tools/mod.rs`
- `src-tauri/src/agent/tools/fs_read.rs`
- `src-tauri/src/agent/tools/fs_write.rs`

### 2.5 Tauri Commands for Phase 2

**File:** `src-tauri/src/agent/mod.rs`

Add commands:

- `start_run(run_id: String) -> Result<(), String>` - Starts loop in background task
- `cancel_run(run_id: String) -> Result<(), String>` - Cancels via CancellationToken
- `reply_permission(run_id: String, permission_id: String, granted: bool) -> Result<(), String>`

**Files to modify:**

- `src-tauri/src/lib.rs` - Register new commands

### 2.6 Restart Safety in Loop

**File:** `src-tauri/src/agent/loop/run_loop.rs`

**CRITICAL: Loop must track decision points for restart safety**

- Store `last_decision_event_id` when emitting decision-related events
- Before each tool execution, emit a marker event (e.g., `DECISION_MADE` with decision details)
- On loop start/resume, query events since last decision point (not by count)

**Restart Safety Checklist:**

- [ ] Loop can be stopped mid-execution
- [ ] On restart, loop detects last completed side-effect
- [ ] Loop does not duplicate file operations
- [ ] Loop does not restart incomplete tool executions
- [ ] Loop resumes from safe point after last confirmed fact event

### 2.7 Exit Criteria for Phase 2

**Test checklist:**

- [ ] Start a run with a goal
- [ ] Run progresses without UI interaction
- [ ] Multiple steps execute
- [ ] Events stream during execution
- [ ] Loop resumes after permission grant
- [ ] **Restart safety**: Kill app mid-run, restart, verify no duplicate side effects
- [ ] **Determinism**: Same events produce same observations (test with event replay)

---

## Phase 3 — Permissions as Loop Gates

### 3.1 Permission Model

**File:** `src-tauri/src/agent/permissions/permission.rs`

```rust
pub struct PermissionRequest {
    pub id: String,
    pub scope: PermissionScope,
    pub reason: String,
    pub risk_score: f32,
    pub scope_type: PermissionScopeType, // CRITICAL: Explicit scope type
}

pub enum PermissionScopeType {
    Once,      // Allow for this single operation only
    Run,       // Allow for the entire run
    // Future: Global, TimeLimited, etc.
}

pub enum PermissionScope {
    FileRead { path: String },
    FileWrite { path: String },
    DirectoryRead { path: String },
    DirectoryWrite { path: String },
}
```

**CRITICAL Permission Rules:**

1. **Permission decisions are events** - `PERMISSION_DECISION` event with granted/denied
2. **Permission memory is scoped to run, not global** - Permissions are stored in RunState, not globally
3. **"Allow once" vs "allow for run" must be encoded explicitly** - Use `PermissionScopeType` enum
4. **Permission state is reconstructed from events** - No separate permission store

**If permissions are stored globally or outside events, the implementation is wrong.**

### 3.2 Permission Flow in Loop

**File:** `src-tauri/src/agent/loop/run_loop.rs`

When tool requires permission:

1. Emit **fact event**: `PERMISSION_REQUESTED` (with scope, reason, scope_type)
2. Emit **projection event**: `RUN_STATUS_CHANGED` → `WaitingPermission`
3. Block loop (wait on channel/condition)
4. On permission decision:

   - Emit **fact event**: `PERMISSION_DECISION` (with permission_id, granted, scope_type)
   - If granted: Resume loop, check scope_type (once vs run)
   - If denied: Replan (call `decide_next` again)

**Permission State Reconstruction:**

- On restart, replay events to reconstruct permission state
- Check `PERMISSION_DECISION` events to determine if permission is still valid
- If scope_type is `Once`, permission is consumed after use
- If scope_type is `Run`, permission persists for entire run

**Files to modify:**

- `src-tauri/src/agent/loop/run_loop.rs` - Add permission blocking logic

---

## Phase 4 — Artifacts as Memory

### 4.1 Artifact Model

**File:** `src-tauri/src/agent/state/run_state.rs`

```rust
pub struct Artifact {
    pub id: String,
    pub kind: ArtifactType,
    pub location: String,
    pub summary: String,
    pub source_step: String,
}

pub enum ArtifactType {
    File,
    Directory,
    Text,
    Image,
}
```

### 4.2 Artifact Extraction & Summarization

**File:** `src-tauri/src/agent/memory/artifacts.rs`

After tool execution:

- Extract artifact metadata
- Generate summary (cheap LLM call or heuristic)
- Store in `RunState`
- Emit `artifact.created`

**Files to create:**

- `src-tauri/src/agent/memory/mod.rs`
- `src-tauri/src/agent/memory/artifacts.rs`

### 4.3 Artifact Retrieval in Observation

**File:** `src-tauri/src/agent/loop/run_loop.rs`

`observe_state()` must:

- Include relevant artifacts in `Observation`
- Filter by relevance (simple keyword matching initially)

---

## Phase 5 — Plans as Derived Projections

### 5.1 Plan Artifact Handling

**File:** `src-tauri/src/agent/memory/artifacts.rs`

- LLM may emit plan artifact (`plan.json`)
- Store as regular artifact
- UI renders it
- Loop ignores it (never executes "according to plan")

**Implementation:**

- No special handling in loop
- Plans are just artifacts with `kind: ArtifactType::Text`

---

## Phase 6 — Recovery & Robustness

### 6.1 Standard Recovery Strategies

**File:** `src-tauri/src/agent/loop/run_loop.rs`

On step failure:

1. Retry with backoff (max 3 attempts)
2. If retry fails: Choose alternate tool (if available)
3. If no alternate: Narrow scope
4. If still fails: Ask user
5. If user unavailable: Fail gracefully

**Default behavior:**

- Step failure ≠ run failure
- Run continues unless explicitly cancelled

---

## Phase 7 — Headless + Reconnect

### 7.1 Background Task Management

**File:** `src-tauri/src/agent/run/store.rs`

- Store active run IDs in memory (Arc<Mutex<HashSet<String>>>)
- Store CancellationToken for each active run (Arc<Mutex<HashMap<String, CancellationToken>>>)
- On app start: Resume all non-terminal runs

**CRITICAL: Restart Safety Implementation**

On app start, for each non-terminal run:

1. **Replay all events** to reconstruct RunState
2. **Detect last completed side-effect:**

   - Find last `TOOL_EXECUTED` event with successful completion
   - Check if corresponding `FILE_WRITTEN` or `FILE_READ` exists
   - If tool executed but file operation missing → incomplete, do not restart tool

3. **Detect current state:**

   - If `PERMISSION_REQUESTED` exists without `PERMISSION_DECISION` → resume waiting
   - If mid-tool execution → handle appropriately (mark incomplete, ask user, or retry)

4. **Continue from safe point:**

   - Resume loop from after last confirmed side-effect
   - Never duplicate side effects

**Restart Safety Rules:**

- Fact events (`TOOL_EXECUTED`, `FILE_WRITTEN`) are emitted BEFORE side effects
- On restart, check fact events to determine completion state
- If side effect is incomplete, do NOT restart it - handle as failure or ask user
- Loop must be idempotent: replaying events should not cause duplicate operations

**If restart logic duplicates side effects or restarts incomplete operations, the implementation is wrong.**

- Loop runs in background Tokio tasks

### 7.2 UI Reconnect

**File:** Frontend agent UI component (to be created)

- On mount: Subscribe to `run_event` Tauri events
- Replay all events for active runs
- Display current state

**Files to create:**

- `src/components/agent/AgentView.tsx` - Main agent UI
- `src/components/agent/RunList.tsx` - List of runs
- `src/components/agent/RunDetail.tsx` - Individual run view

---

## Implementation Order

1. **Phase 0** - Module structure + UI mode selection
2. **Phase 1** - Run + Event log (test thoroughly)
3. **Phase 2** - Minimal loop (test thoroughly)
4. **Phase 3** - Permissions
5. **Phase 4** - Artifacts
6. **Phase 5** - Plans (minimal)
7. **Phase 6** - Recovery
8. **Phase 7** - Headless + reconnect

---

## Critical Implementation Rules

1. **Never store state directly** - Only events
2. **Never let UI mutate run state** - Only events
3. **Never let LLM drive the loop** - LLM is advisory only
4. **Never shortcut event sourcing** - All state from events
5. **Isolate agent module** - No mixing with assistant logic

---

## Dependencies to Add

**Cargo.toml additions:**

- `tokio-util = { version = "0.7", features = ["sync"] }` - For CancellationToken
- No new major dependencies needed (use existing `tauri-plugin-sql`, `serde_json`, `chrono`)

---

## Testing Strategy

Each phase must pass exit criteria before proceeding. Manual testing is acceptable for now:

- Create runs
- Append events
- Restart app
- Verify state reconstruction
- Test loop execution
- Test permission flow

---

## Critical Design Distinctions (Summary)

### Facts vs Projections

**Facts (Ontology - What Happened):**

- `TOOL_EXECUTED`, `FILE_WRITTEN`, `FILE_READ`, `PERMISSION_REQUESTED`, `PERMISSION_DECISION`
- These describe actual occurrences
- Cannot be changed or reinterpreted
- Emitted BEFORE side effects (for restart safety)

**Projections (Interpretations - What It Means):**

- `STEP_STARTED`, `STEP_COMPLETED`, `STEP_FAILED`, `RUN_STATUS_CHANGED`, `ARTIFACT_CREATED`
- These are derived from facts
- Can be recalculated from facts
- Emitted AFTER side effects complete

**If Cursor treats projection events as authoritative or emits them before facts, the implementation is wrong.**

### RunState vs Events

**Events:**

- Authoritative source of truth
- Append-only, immutable
- Reconstructable state

**RunState:**

- Derived projection/view
- Convenience for UI and loop
- Can be recalculated from events
- Multiple projections possible (timeline, audit, debug)

**If Cursor mutates RunState directly or treats it as authoritative, the implementation is wrong.**

### Determinism Requirements

**recent_events:**

- Must be defined by event IDs since last decision
- NOT by count or time window
- Ensures reproducible LLM inputs

**Restart Safety:**

- Must detect last completed side-effect
- Must not duplicate operations
- Must resume from safe point
- Fact events emitted BEFORE side effects enable this

**If recent_events uses count-based selection or restart duplicates side effects, the implementation is wrong.**

### Permission Scoping

**Permissions:**

- Stored in RunState (reconstructed from events)
- Scoped to run, not global
- Explicit scope_type: `Once` vs `Run`
- Permission decisions are events

**If permissions are global or stored outside events, the implementation is wrong.**