/**
 * Context window management: trim conversation history to stay within
 * message count and token budget for stable latency and cost.
 */
import {
  MAX_HISTORY_MESSAGES,
  MAX_CONTEXT_TOKENS,
  CHARS_PER_TOKEN_ESTIMATE,
} from "./chat-constants";

export interface MessageLike {
  content: string;
  [key: string]: unknown;
}

function estimateTokens(text: string): number {
  return Math.ceil((text || "").length / CHARS_PER_TOKEN_ESTIMATE);
}

/**
 * Trim history to fit within MAX_HISTORY_MESSAGES and MAX_CONTEXT_TOKENS.
 * Keeps the most recent messages (from the end of the array).
 */
export function trimHistoryToContextWindow<T extends MessageLike>(
  messages: T[]
): T[] {
  if (messages.length <= MAX_HISTORY_MESSAGES) {
    let total = 0;
    for (let i = messages.length - 1; i >= 0; i--) {
      total += estimateTokens(messages[i].content);
      if (total > MAX_CONTEXT_TOKENS) {
        return messages.slice(i + 1);
      }
    }
    return messages;
  }
  const byCount = messages.slice(-MAX_HISTORY_MESSAGES);
  let total = 0;
  for (let i = byCount.length - 1; i >= 0; i--) {
    total += estimateTokens(byCount[i].content);
    if (total > MAX_CONTEXT_TOKENS) {
      return byCount.slice(i + 1);
    }
  }
  return byCount;
}
