# How to Define a New Capability

This doc shows the **exact** structure, parameters, and patterns. Use it when adding a new capability (e.g. `clipboard.copy`, `http.get`, `database.query`).

---

## 1. The contract: `Capability` trait

Every capability implements this trait (defined in `src/agent/capabilities/mod.rs`):

```rust
#[async_trait]
pub trait Capability: Send + Sync {
    fn name(&self) -> &'static str;                    // required
    fn description(&self) -> &'static str;             // required
    fn side_effects(&self) -> &'static [&'static str]; // required
    fn risk_level(&self) -> RiskLevel;                  // default: Medium
    fn requires_permission(&self) -> bool;             // default: true
    fn artifacts_produced(&self) -> &'static [&'static str]; // default: &[]
    fn input_schema(&self) -> Value;                   // required (JSON Schema)
    fn preflight(&self, inputs: &Value) -> PreflightResult; // default: Ok
    async fn execute(&self, ctx: CapabilityContext, inputs: Value) -> Result<CapabilityResult, String>; // required
}
```

**Types you use:**

| Type | Where | Meaning |
|------|--------|--------|
| `Value` | `serde_json::Value` | JSON value (object for inputs/schema) |
| `PreflightResult` | `capabilities::mod` | `Ok` \| `NeedsPermission(PermissionRequest)` \| `NeedsInput(InputRequest)` \| `Reject(String)` |
| `InputRequest` | `capabilities::mod` | `{ missing_fields: Vec<String>, schema: Value, current_inputs: Value }` |
| `PermissionRequest` | `capabilities::mod` | `{ reason: String }` |
| `CapabilityContext` | `capabilities::mod` | `{ app: AppHandle, run_id: String, state: RunState }` |
| `CapabilityResult` | `capabilities::mod` | `{ outcome: CapabilityOutcome, artifacts: Vec<Value>, side_effects: Vec<String> }` |
| `CapabilityOutcome` | `capabilities::mod` | `Success` \| `Partial` \| `Failure(String)` |
| `RiskLevel` | `capabilities::mod` | `Low` \| `Medium` \| `High` \| `Critical` |

---

## 2. Parameter reference

### `name() -> &'static str`
- **Unique** ID used by the planner and registry (e.g. `"filesystem.read"`, `"process.spawn"`).
- Use a **dot** namespace: `"domain.action"`.

### `description() -> &'static str`
- One line for the LLM and UI (e.g. "Read contents of a file from disk").

### `side_effects() -> &'static [&'static str]`
- List of **effect tags** (e.g. `&["disk_read"]`, `&["disk_write"]`, `&["process_execution"]`).
- Used for auditing and permission UX.

### `risk_level() -> RiskLevel`
- `Low` / `Medium` / `High` / `Critical`. Affects auto-approval and permission UI.

### `requires_permission() -> bool`
- `true`: run loop will show a **permission** step after input (if not auto-approved).
- `false`: after user provides input, execution runs immediately (e.g. `filesystem.write`).

### `artifacts_produced() -> &'static [&'static str]`
- Kinds of artifacts (e.g. `&["file"]`, `&["process_output"]`). Informational.

### `input_schema() -> Value`
- **JSON Schema** for inputs. Must have `type: "object"`, `properties`, and `required`.
- The **run loop** uses `required` + generic “empty” checks to decide if it should ask for input.
- The **UI** uses this to render the Input Request dialog (labels, types, descriptions).

**Schema shape (minimal):**

```json
{
  "type": "object",
  "properties": {
    "your_field": {
      "type": "string",
      "description": "Human-readable hint for LLM and UI"
    }
  },
  "required": ["your_field"]
}
```

Supported `properties.*.type`: `string`, `number`, `boolean`; arrays/objects as needed.

### `preflight(&self, inputs: &Value) -> PreflightResult`
- **No I/O.** Pure validation.
- Return:
  - **`PreflightResult::Ok`** → proceed (or to permission if `requires_permission`).
  - **`PreflightResult::NeedsInput(InputRequest { missing_fields, schema, current_inputs })`** → ask user for missing/invalid fields (e.g. empty path, empty content).
  - **`PreflightResult::NeedsPermission(PermissionRequest { reason })`** → show permission step (unless auto-approved).
  - **`PreflightResult::Reject("message")`** → fail the step (no execution).

Use **`self.input_schema()`** for `InputRequest.schema` and **`inputs.clone()`** for `current_inputs`.

### `execute(&self, ctx: CapabilityContext, inputs: Value) -> Result<CapabilityResult, String>`
- Do the real work. You get:
  - **`ctx.app`**: Tauri `AppHandle` (for `store::append_event`, dialogs, etc.).
  - **`ctx.run_id`**: current run.
  - **`ctx.state`**: current run state (goal, etc.).
- **Conventions:**
  1. Emit **`TOOL_EXECUTED`** (with capability name and inputs) **before** the side effect.
  2. Perform the side effect (read file, write file, spawn process).
  3. Emit domain events (e.g. **`FILE_READ`**, **`FILE_WRITTEN`**) and **`STEP_COMPLETED`**.
  4. If you produce an artifact, emit **`ARTIFACT_CREATED`**.
  5. Return **`Ok(CapabilityResult { outcome, artifacts, side_effects })`** or **`Err(message)`**.

---

## 3. File layout for a new capability

Option A: add to an existing domain (e.g. `filesystem`):

```
src/agent/capabilities/
  filesystem/
    mod.rs      // pub use read::...; pub use write::...; pub use your_new::YourNew;
    read.rs
    write.rs
    your_new.rs   // your new capability
```

Option B: new domain (e.g. `clipboard`):

```
src/agent/capabilities/
  mod.rs       // add: pub mod clipboard;
  clipboard/
    mod.rs     // pub use copy::ClipboardCopy;
    copy.rs    // struct ClipboardCopy; impl Capability for ClipboardCopy { ... }
  registry.rs
```

Then in **`lib.rs`** (in the `setup` spawn where the registry is filled):

```rust
registry.register(Arc::new(agent::capabilities::filesystem::YourNew)).await;
// or
registry.register(Arc::new(agent::capabilities::clipboard::ClipboardCopy)).await;
```

---

## 4. Minimal skeleton (new capability)

```rust
// capabilities/example/echo.rs

use crate::agent::capabilities::{Capability, CapabilityContext, CapabilityOutcome, CapabilityResult, PreflightResult, RiskLevel};
use crate::agent::events::TOOL_EXECUTED;
use crate::agent::run::store;
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ExampleEcho;

#[async_trait]
impl Capability for ExampleEcho {
    fn name(&self) -> &'static str {
        "example.echo"
    }

    fn description(&self) -> &'static str {
        "Echo a message (example capability)"
    }

    fn side_effects(&self) -> &'static [&'static str] {
        &[]
    }

    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }

    fn requires_permission(&self) -> bool {
        false
    }

    fn artifacts_produced(&self) -> &'static [&'static str] {
        &[]
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo"
                }
            },
            "required": ["message"]
        })
    }

    fn preflight(&self, inputs: &Value) -> PreflightResult {
        let msg = inputs.get("message").and_then(|v| v.as_str()).unwrap_or("").trim();
        if msg.is_empty() {
            return PreflightResult::NeedsInput(crate::agent::capabilities::InputRequest {
                missing_fields: vec!["message".to_string()],
                schema: self.input_schema(),
                current_inputs: inputs.clone(),
            });
        }
        PreflightResult::Ok
    }

    async fn execute(
        &self,
        ctx: CapabilityContext,
        inputs: Value,
    ) -> Result<CapabilityResult, String> {
        let message = inputs["message"]
            .as_str()
            .ok_or_else(|| "Missing message".to_string())?;

        store::append_event(
            &ctx.app,
            &ctx.run_id,
            TOOL_EXECUTED,
            json!({
                "capability": self.name(),
                "inputs": inputs,
                "output": format!("Echo: {}", message)
            }),
        )
        .await?;

        Ok(CapabilityResult {
            outcome: CapabilityOutcome::Success,
            artifacts: vec![json!({ "echoed": message })],
            side_effects: vec![],
        })
    }
}
```

---

## 5. Full real example: `filesystem.read`

Location: **`src/agent/capabilities/filesystem/read.rs`**.

- **name:** `"filesystem.read"`
- **description:** "Read contents of a file from disk"
- **side_effects:** `["disk_read"]`
- **risk_level:** `Low`
- **requires_permission:** `true`
- **artifacts_produced:** `["file"]`
- **input_schema:** one required field `path` (string).
- **preflight:** if `path` is empty → `NeedsInput` with `missing_fields: ["path"]`; else if permission required → `NeedsPermission("Read file: ...")`; else `Ok`.
- **execute:**
  1. Emit `TOOL_EXECUTED`.
  2. `fs::read_to_string(path)`.
  3. Emit `FILE_READ` (path, content_hash, size).
  4. Emit `STEP_COMPLETED`.
  5. Emit `ARTIFACT_CREATED` (File artifact with content preview).
  6. Return `CapabilityResult { outcome: Success, artifacts: [...], side_effects: ["disk_read"] }`.

This is the exact pattern to copy for read-only, single-input capabilities that need permission.

---

## 6. Checklist for a new capability

1. Create the struct and `impl Capability` in a new file (or under an existing domain `mod`).
2. Implement **name**, **description**, **side_effects**, **risk_level**, **requires_permission**, **artifacts_produced**, **input_schema**, **preflight**, **execute**.
3. In **preflight**: return `NeedsInput` when required fields are missing or invalid; use `self.input_schema()` and `inputs.clone()` in `InputRequest`.
4. In **execute**: emit `TOOL_EXECUTED` before the side effect; then domain events and `STEP_COMPLETED`; then return `CapabilityResult`.
5. Export the struct from the parent `mod.rs` (e.g. `pub use read::FilesystemRead`).
6. In **`lib.rs`** (setup): `registry.register(Arc::new(agent::capabilities::your_module::YourCapability)).await;`
7. Rebuild; the planner and run loop will pick it up by **name** from the registry.
