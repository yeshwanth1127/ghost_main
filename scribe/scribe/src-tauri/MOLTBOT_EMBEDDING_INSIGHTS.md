# Moltbot vs Scribe: Embedding / Intent Routing Insights

This doc summarizes how **moltbot** handles intent and embeddings, and what we can reuse for scribe’s embedding-based router (fast path vs LLM).

---

## Does Moltbot Do the Same Process?

**No.** Moltbot does **not** do embedding-based routing to skip the LLM.

- **Scribe (goal):** User goal → **router** (embedding similarity) → **direct** (parse + execute) **or** **LLM** (planner).
- **Moltbot (actual):** User message → **always Pi agent (LLM)** → LLM requests tools → gateway creates **execution tickets** → permission → executor runs. There is no pre-LLM “fast path” that parses the goal and runs a direct command.

So we cannot copy “the exact process” from moltbot for routing; we can only borrow patterns and alternatives.

---

## What Moltbot Does Do

### 1. Post-LLM intent canonicalization

**Location:** `moltbot/src/gateway/intent/`

- **When:** After the LLM has already chosen a tool and the gateway receives a `tool.request`.
- **What:** Converts raw LLM intent into a structured **ToolIntent** (risk factors, human-readable description, goal alignment, irreversible flag) for permissions, UI, and auditing.
- **Files:** `canonicalizer.ts`, `memory-enhanced.ts`, `types.ts`.

**Relevant patterns for scribe:**

- **Risk factors:** e.g. `system_path`, `home_directory`, `destructive_command`, `process_execution` (see `canonicalizer.ts`).
- **Goal alignment:** Derive from `rawIntent` text (e.g. “create file”, “run command”) or from `sessionContext.userGoal`.
- **Human-readable description:** e.g. “Create file at X to achieve Y” so permissions/UI stay consistent.

You can align scribe’s intent names or descriptions with these concepts where it helps.

### 2. Memory-enhanced canonicalization (optional)

**Location:** `moltbot/src/gateway/intent/memory-enhanced.ts`

- **When:** Same as above; optional enrichment when a memory manager is available.
- **What:** Builds a search query like `tool: ${tool} goal: ${userGoal}`, runs **memory search**, and attaches “similar past intents” to the ToolIntent. Still **post-LLM**; not used for routing.

### 3. Embeddings: memory search only (no routing)

**Location:** `moltbot/src/memory/`

- **Use:** Semantic search over session/memory content (chunk → embed → store in SQLite with sqlite-vec; query → embed → vector search). Used for “similar past intents” and general memory retrieval.
- **Not used for:** Deciding “direct vs LLM” or any pre-LLM routing.

**Embedding providers (moltbot):**

| Provider | How |
|----------|-----|
| **OpenAI** | `embeddings-openai.ts` – API (e.g. `text-embedding-3-small`). |
| **Gemini** | `embeddings-gemini.ts` – API (e.g. `gemini-embedding-001`). |
| **Local** | `embeddings.ts` + **node-llama-cpp** – GGUF models (e.g. `embeddinggemma-300M`). No ONNX. |

**Abstraction:** `EmbeddingProvider` with `embedQuery(text)` and `embedBatch(texts)`. One implementation per backend; “auto” tries local then OpenAI then Gemini.

---

## Insights for Scribe

### 1. Moltbot does not implement “embedding router → direct path”

So the “exact process” (pre-LLM routing using embeddings) is something scribe implements on its own; moltbot is a different design (always LLM, then tickets).

### 2. Optional API-based embeddings for the router (avoid ONNX on Windows)

To avoid ONNX/ORT on Windows you could:

- Add an **optional** embedding backend for the router: e.g. **OpenAI** or **Gemini** (same idea as moltbot’s memory embeddings).
- Keep ONNX as the default/local path where it works; use API when `EMBEDDING_MODEL_PATH` / ORT are not set or fail.
- Requires an API key and network; no local DLL or model.

Implementation sketch: abstract “embedder” in scribe (trait in Rust) with (a) existing ONNX impl, (b) optional HTTP client for OpenAI/Gemini embed endpoint. Router calls the abstract embedder; config or env chooses provider.

### 3. Provider abstraction

Moltbot’s `EmbeddingProvider` + `createEmbeddingProvider(options)` with `openai | local | gemini | auto` and fallback is a good pattern. Scribe could have:

- `EmbeddingRouterProvider`: ONNX (local), optional OpenAI, optional Gemini.
- Same interface: “embed this string” / “embed these strings”; router code stays agnostic.

### 4. Intent taxonomy and canonicalization

- Reuse **risk / goal alignment** ideas from `canonicalizer.ts` when you expose intent in the UI or in permissions (e.g. “file_operation” vs “run_command” with risk flags).
- You can keep scribe’s current intent set; optionally align labels/descriptions with moltbot’s human-readable style for consistency across products.

### 5. Local embeddings in moltbot (no ONNX)

Moltbot uses **node-llama-cpp** + GGUF for local embeddings, not ONNX. If scribe ever wanted “local, no API” without ONNX, that would mean a different stack (e.g. llama.cpp bindings + small embedding GGUF). That’s a larger change; for now ONNX + optional API is a simpler path.

---

## Summary Table

| Aspect | Moltbot | Scribe (current) |
|--------|---------|-------------------|
| Pre-LLM routing (direct vs LLM) | No | Yes (embedding router) |
| Where embeddings are used | Memory search, post-LLM enrichment | Intent routing (pre-LLM) |
| Embedding stack | OpenAI / Gemini / node-llama-cpp (GGUF) | ONNX (all-MiniLM-L6-v2) + load-dynamic on Windows |
| Intent canonicalization | Post-LLM (tool intent for permissions/UI) | Pre-LLM (intent label + confidence for routing) |
| “Fast path” | No (all tools via LLM → tickets) | Yes (direct parse + execute when confidence high) |

Use moltbot for **patterns** (provider abstraction, API/local options, canonicalization and risk) rather than for copying the same routing flow.
