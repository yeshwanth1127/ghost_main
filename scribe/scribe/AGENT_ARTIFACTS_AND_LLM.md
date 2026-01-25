# Agent Artifacts and LLM Integration

## How Artifacts Are Used After Storage

### 1. Artifact Storage Flow

When a tool executes (e.g., `fs_read` or `fs_write`):

1. **Tool Execution** → Emits fact events (`TOOL_EXECUTED`, `FILE_READ`/`FILE_WRITTEN`)
2. **Artifact Creation** → Tool creates an `Artifact` struct with:
   - `id`: Unique identifier
   - `kind`: ArtifactType (File, Directory, Text, Image)
   - `location`: Path or location
   - `summary`: Content preview (first 500 chars for files) + metadata
   - `source_step`: Step ID that created it
   - `created_at`: Timestamp

3. **Event Emission** → Emits `ARTIFACT_CREATED` projection event
4. **State Update** → Artifact is added to `RunState.artifacts` via reducer

### 2. Artifact Usage in Decision-Making

Artifacts are used in the **observation phase** before each LLM decision:

**Location:** `src-tauri/src/agent/loop/run_loop.rs::decide_next()`

**Process:**
1. All artifacts from `RunState` are included in the `Observation` struct
2. Artifacts are formatted into a context string for the LLM:
   ```
   AVAILABLE ARTIFACTS (use these to inform your decisions):
   - File: /path/to/file.txt (Read file: /path/to/file.txt (1234 bytes). Content preview: ...)
   - File: /path/to/other.txt (Wrote file: /path/to/other.txt (567 bytes))
   ```
3. The LLM receives artifacts in the system prompt with instructions:
   - "Review artifacts before taking actions - they show what has already been done"
   - "Use artifacts to avoid redundant operations"
   - "Consider the goal and current artifacts when deciding the next step"

### 3. Artifact Content Preview

For `fs_read` operations:
- Artifact summary includes first 500 characters of file content
- This allows LLM to understand what was read without storing full content
- Format: `"Read file: {path} ({size} bytes). Content preview: {first_500_chars}..."`

For `fs_write` operations:
- Artifact summary includes file path and size
- Format: `"Wrote file: {path} ({size} bytes)"`

### 4. Artifact Filtering (Future Enhancement)

Currently, all artifacts are included in observations. The `get_relevant_artifacts()` function in `memory/artifacts.rs` provides keyword-based filtering for future use.

---

## LLM Request Flow Through Existing Chat Infrastructure

### Route: Agent → Existing Chat API

**Location:** `src-tauri/src/agent/loop/run_loop.rs::decide_next()`

**Flow:**
```
Agent decide_next()
  ↓
api::chat_stream()  ← Uses existing chat infrastructure
  ↓
/api/v1/chat?stream=true  ← Same endpoint as chat mode
  ↓
OpenRouter/LLM Provider
  ↓
Streaming response (SSE)
  ↓
Parsed JSON decision
```

### Key Points:

1. **Same Infrastructure**: Uses `api::chat_stream()` which is the same function used by chat mode
2. **Same Endpoint**: Routes to `/api/v1/chat?stream=true` (scribe-api server)
3. **Same Authentication**: Uses license key, instance ID, machine ID headers
4. **Same Model Selection**: Uses the user's selected model from settings
5. **Same Streaming**: Uses Server-Sent Events (SSE) streaming
6. **Same Error Handling**: Uses the same error handling and retry logic

### Request Details:

**Function Call:**
```rust
api::chat_stream(
    app.clone(),
    user_message,           // "Based on the goal '...', what should I do next?"
    Some(system_prompt),    // Contains goal, artifacts, events, available actions
    None,                   // No image for agent decisions
    None,                   // No history - each decision is independent
)
```

**HTTP Request:**
- Method: POST
- URL: `{APP_ENDPOINT}/api/v1/chat?stream=true`
- Headers:
  - `Authorization: Bearer {API_ACCESS_KEY}`
  - `license_key: {LICENSE_KEY}`
  - `instance: {INSTANCE_ID}`
  - `provider: {PROVIDER}`
  - `model: {MODEL}`
  - `machine_id: {MACHINE_ID}`
- Body: `ChatRequest` with user_message, system_prompt, etc.

**Response Handling:**
- Streams SSE events
- Extracts text content from various response formats
- Returns full response as String
- Parsed as JSON to extract decision

### Why This Design:

1. **Consistency**: Agent and chat mode use the same LLM infrastructure
2. **Model Selection**: User's model preference applies to both modes
3. **License Management**: Same license validation and usage tracking
4. **Error Handling**: Same error handling and retry logic
5. **Streaming**: Same streaming infrastructure for real-time responses

---

## Example: Artifact Usage in Decision Loop

### Scenario: Agent needs to read a config file, then write a new file

**Step 1: LLM decides to read config**
- Observation: No artifacts yet
- Decision: `fs_read` with path to config file
- Tool executes, creates artifact:
  ```
  Artifact {
    location: "/path/to/config.json",
    summary: "Read file: /path/to/config.json (234 bytes). Content preview: {\"key\": \"value\"...}"
  }
  ```

**Step 2: LLM decides next action**
- Observation: Contains artifact from step 1
- System prompt includes:
  ```
  AVAILABLE ARTIFACTS:
  - File: /path/to/config.json (Read file: /path/to/config.json (234 bytes). Content preview: {"key": "value"...})
  ```
- LLM can see the config was already read and use that information
- Decision: `fs_write` to create new file based on config content

**Step 3: LLM sees both artifacts**
- Observation: Contains both read and write artifacts
- LLM can verify the goal is complete
- Decision: `finish`

---

## Artifact Memory Benefits

1. **Avoid Redundancy**: LLM knows what files were already read/written
2. **Context Awareness**: LLM can reference previous file contents in summaries
3. **Progress Tracking**: Artifacts show what has been accomplished
4. **Decision Quality**: Better decisions when LLM knows what's already been done

---

## Technical Notes

- Artifacts are stored in `RunState` (derived projection from events)
- Artifacts persist across app restarts (reconstructed from events)
- Artifact summaries are lightweight (content previews, not full content)
- Full file content is not stored in artifacts to keep state manageable
- LLM receives artifact summaries in every decision prompt
