# Capabilities button (no LLM)

## Goal

When the user wants to know what the agent/app can do, show the answer **directly** via a button—no request sent to the LLM, no tokens, no delay.

## Approach

1. **Single source of truth**  
   Add a small content module (e.g. `scribe/scribe/src/config/ghost-capabilities.ts`) that exports a string constant with Ghost’s capabilities: chat, Agent (Pi) mode, filesystem/process, direct path, gateway, etc. (~150–250 words).

2. **Button in the chat UI**  
   Add a button near the input (e.g. in [scribe/scribe/src/components/completion/index.tsx](scribe/scribe/src/components/completion/index.tsx) next to the logo/input, or inside [Input.tsx](scribe/scribe/src/components/completion/Input.tsx) / [MediaGroup.tsx](scribe/scribe/src/components/completion/MediaGroup.tsx)) labeled e.g. “What can Ghost do?” or “Capabilities”.

3. **Show directly on click**  
   On click, **do not** send any message to the LLM. Instead:
   - Either **append the capabilities text as an assistant message** to the current conversation (reuse existing “assistant message” UI so it looks like a reply), or
   - **Open a small modal/drawer** that displays the same text.

   Preferred: append as assistant message so it lives in the same thread and feels like an instant answer.

4. **Implementation details**
   - **If appending as assistant message**: Use the same conversation state that `useCompletion` uses (e.g. `conversationHistory` / `saveCurrentConversation` or equivalent). Add a helper like `appendAssistantMessage(content: string)` that adds one assistant message with `content` and updates state; call it with `GHOST_CAPABILITIES_KNOWLEDGE` when the button is clicked. No `fetchAIResponse`, no `submit`.
   - **If modal**: A simple modal component that receives the capabilities string and displays it; button opens it. No conversation state change.

## Files to add/change

| Action | File |
|--------|------|
| Add | `scribe/scribe/src/config/ghost-capabilities.ts` – export capabilities text |
| Edit | Chat/completion UI (e.g. `Completion` or `MediaGroup` / `Input`) – add “What can Ghost do?” button |
| Edit | `scribe/scribe/src/hooks/useCompletion.ts` (or equivalent) – add a function to append an assistant message to the current conversation without calling the LLM, and expose it for the button |

## Out of scope

- No system-prompt augmentation for “what can you do?” (no LLM involved for this path).
- No RAG or retrieval.

## Result

- User clicks “What can Ghost do?” → capabilities text appears immediately (as assistant message or in modal), no API call, no delay.
