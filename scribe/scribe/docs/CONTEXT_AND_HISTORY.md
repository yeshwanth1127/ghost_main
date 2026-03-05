# How chat context is saved and used

This doc describes the **actual implementation** of conversation context in Ghost (Chat mode): where it is stored, when it is saved, and how it is sent to the model for follow-up responses.

---

## 1. Where context lives

### In memory (current session)

- **Hook state**: [useCompletion.ts](src/hooks/useCompletion.ts) keeps:
  - `conversationHistory: ChatMessage[]` – list of `{ id, role, content, timestamp }` for the **current** conversation
  - `currentConversationId: string | null` – id of the active conversation (null = new chat)
- When you send a message, the **history** passed to the AI is exactly `state.conversationHistory` mapped to `{ role, content }` (no ids/timestamps).

### On disk (persistence)

- **SQLite** via Tauri plugin: `sqlite:ghost.db`
- **Tables** (created by backend migrations in [src-tauri/src/db/migrations/chat-history.sql](src-tauri/src/db/migrations/chat-history.sql)):
  - `conversations`: `id`, `title`, `created_at`, `updated_at`
  - `messages`: `id`, `conversation_id`, `role`, `content`, `timestamp`, `attached_files`
- **Frontend API**: [src/lib/database/chat-history.action.ts](src/lib/database/chat-history.action.ts)
  - `saveConversation(conversation)` – upsert: create new or update existing (replace all messages for that conversation)
  - `getConversationById(id)` – load one conversation with all messages
  - `getAllConversations()` – list all conversations (for sidebar)
  - `deleteConversation(id)` / `deleteAllConversations()` – delete data
- **DB access**: [src/lib/database/config.ts](src/lib/database/config.ts) uses `@tauri-apps/plugin-sql` and `getDatabase()` to get the same `ghost.db` instance (backend runs migrations; frontend reads/writes via the plugin).

So: **context is really saved** in SQLite; it is not only in memory.

---

## 2. When context is saved

1. **After a successful AI reply (Chat mode)**  
   In [useCompletion.ts](src/hooks/useCompletion.ts), inside `submit()`:
   - When `fetchAIResponse` finishes and `fullResponse` is non-empty, it calls **`saveCurrentConversation(input, fullResponse, attachments)`**.
   - That builds:
     - `userMsg` = current user message
     - `assistantMsg` = full AI response
     - `newMessages = [...state.conversationHistory, userMsg, assistantMsg]`
   - Then it calls **`saveConversation(conversation)`** with:
     - `id`: `state.currentConversationId` or a new `generateConversationId("chat")`
     - `title`: from first user message or existing conversation title
     - `messages`: `newMessages`
   - So **each successful user→assistant turn is persisted** (create or update that conversation in SQLite).
   - After save, state is updated: `currentConversationId`, `conversationHistory: newMessages`, and input/attachments are cleared.

2. **When user clicks "What can I do?" (capabilities)**  
   Same flow: `showCapabilities()` builds a synthetic user + assistant pair, calls `saveConversation`, and updates `conversationHistory` and `currentConversationId`.

3. **Legacy**  
   Old localStorage chat history can be migrated once into SQLite via `migrateLocalStorageToSQLite()` (called from [useApp.ts](src/hooks/useApp.ts) on startup).

---

## 3. How context is used for the next response

1. **Building the history sent to the AI**  
   In `submit()` in [useCompletion.ts](src/hooks/useCompletion.ts):

   ```ts
   const messageHistory = state.conversationHistory.map((msg) => ({
     role: msg.role,
     content: msg.content,
   }));
   ```

   So **every message in the current conversation** (all previous user + assistant turns in this thread) is sent as `history` to the AI.

2. **Scribe API path (Tauri → scribe-api)**  
   In [ai-response.function.ts](src/lib/functions/ai-response.function.ts), `fetchScribeAIResponse`:
   - Converts `history` to the format expected by the backend: `[{ role, content: [{ type: "text", text: msg.content }] }]` (reversed for the API).
   - Passes it as `history: historyString` to `invoke("chat_stream", { userMessage, systemPrompt, imageBase64, history })`.
   - So the **same conversation history** is sent to the chat API for context.

3. **Direct provider path (e.g. OpenRouter / Ollama)**  
   In `fetchAIResponse`, when not using Scribe API:
   - The request body is built from the provider’s cURL template.
   - **`buildDynamicMessages(bodyObj.messages, history, userMessage, imagesBase64)`** ([common.function.ts](src/lib/functions/common.function.ts)) is used:
     - It finds the template slot for the user message (e.g. `{{TEXT}}`).
     - It builds: `[...prefixMessages, ...history, newUserMessage, ...suffixMessages]`.
   - So **`history` (all previous messages in the conversation) is injected into the messages array** sent to the provider; the model sees the full thread.

So: **context is used** by sending the current conversation’s message list (roles + contents) as the `history` argument to both Scribe API and direct providers; the new user message is appended to that history.

---

## 4. Loading a past conversation (switching context)

- When the user picks a conversation from the sidebar, the app emits a **`conversationSelected`** event with that conversation’s `id`.
- In [useCompletion.ts](src/hooks/useCompletion.ts), a listener calls **`getConversationById(id)`** (SQLite), then **`loadConversation(conversation)`**.
- `loadConversation` sets state:
  - `currentConversationId = conversation.id`
  - `conversationHistory = conversation.messages`
  - and clears input, response, error, loading.
- The next time the user sends a message, **`messageHistory`** is derived from this loaded `conversationHistory`, so the model gets the full thread of the **selected** conversation.

---

## 5. Summary flow

```text
User sends message
  → messageHistory = state.conversationHistory (current thread)
  → fetchAIResponse({ history: messageHistory, userMessage, ... })
      → Scribe: history passed to chat_stream
      → Direct: buildDynamicMessages(..., history, userMessage, ...)
  → AI streams reply
  → saveCurrentConversation(userMsg, assistantMsg)
      → saveConversation() → SQLite (create or update conversation)
  → setState({ conversationHistory: newMessages, currentConversationId })

User selects past conversation
  → getConversationById(id) → SQLite
  → loadConversation(conversation) → setState({ conversationHistory: conversation.messages, ... })
  → Next send uses that thread as history
```

So: **yes, there is a real implementation**: context is stored in SQLite (`conversations` + `messages`), saved after each successful AI reply (and for capabilities), and used by sending the current conversation’s messages as `history` to the AI on every request.

---

## 6. Architecture roadmap: conceptual improvements

Current system is **solid**: real persistence, thread isolation, deterministic history injection, no hidden “memory”. **Already built:** Save user message immediately; context window (trim to last 24 messages + token budget); conversation metadata (model_used, total_tokens); semantic memory (conversation_facts table, get/setFactForConversation, injected into system prompt). No automatic fact extraction yet. Below are **optional** next steps.

### 6.1 Context window management (scalability)

**Today:** Full conversation thread is sent on every request.

**Issue:** Long threads → more tokens, slower responses, higher cost, eventual context limits.

**Improvement:** Context window management:
- Send: system prompt + **recent N messages** + **compressed summary of older messages**.
- Outcome: stable latency, stable token size, effectively unbounded conversation length.

**Priority:** #1 long-term stability gain.

---

### 6.2 Semantic / structured memory (intelligence)

**Today:** Memory is linear chat history only.

**Issue:** References like “use the API key I mentioned earlier” fail if that message was summarized away or is far back.

**Improvement:** Semantic memory layer:
- Extract important facts (preferences, paths, keys, decisions).
- Store separately (key–value or embeddings).
- Inject only **relevant** memories into the prompt.

**Priority:** #1 intelligence boost.

---

### 6.3 Save user message immediately (robustness)

**Today:** Conversation is saved only after the **full** assistant response.

**Issue:** Crash, network failure, or closed window mid-stream loses the turn.

**Improvement:** Save user message as soon as it’s sent; update with assistant message when the stream completes.

**Priority:** Small but clear robustness gain.

---

### 6.4 Token budget awareness (reliability)

**Today:** History is sent without size checks.

**Improvement:** Before each request: estimate tokens, trim or summarize to stay within a budget. Prevents context-overflow and provider errors.

**Priority:** #1 reliability improvement.

---

### 6.5 Conversation metadata layer (routing & UX)

**Today:** Conversations have `id`, `title`, `messages`.

**Improvement:** Optional metadata: model used, total tokens, last intent, context summary, tags (e.g. project, coding, research). Enables smarter routing, faster continuation, better filtering, and analytics.

---

### 6.6 Multi-layer memory (future agent mode)

**Today:** Context = a single message list sent to the model.

**Advanced:** Treat context as structured state: working memory, task memory, semantic memory, ephemeral memory. Relevant when Chat evolves into a more capable agent.

---

### Ranked top 3

1. **Automatic summarization for long chats** – biggest long-term stability.
2. **Lightweight structured memory (fact extraction)** – biggest intelligence gain.
3. **Token budget management** – biggest reliability gain.

---

**Summary:** Context window, token budget, save-user-now, metadata, and semantic facts are implemented. Optional next: automatic summarization and fact extraction.
