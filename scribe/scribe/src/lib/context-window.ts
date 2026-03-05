/**
 * Context window management: trim conversation history to stay within
 * message count and token budget for stable latency and cost.
 */
import { MAX_HISTORY_MESSAGES } from "./chat-constants";
import { getContextBudget } from "@/config/model-context";
import {
  estimateMessageTokens,
  type MessageLike,
} from "./utils/token-counter";

/**
 * Trim history to fit within message count and token budget.
 * Keeps the most recent messages (from the end of the array).
 * Uses token_count on messages when available to avoid re-tokenizing.
 */
export function trimHistoryToContextWindow<T extends MessageLike>(
  messages: T[],
  modelId?: string,
  maxTokens?: number
): T[] {
  const budget = maxTokens ?? getContextBudget(modelId);

  if (messages.length <= MAX_HISTORY_MESSAGES) {
    let total = 0;
    for (let i = messages.length - 1; i >= 0; i--) {
      total += estimateMessageTokens(messages[i]);
      if (total > budget) {
        return messages.slice(i + 1);
      }
    }
    return messages;
  }

  const byCount = messages.slice(-MAX_HISTORY_MESSAGES);
  let total = 0;
  for (let i = byCount.length - 1; i >= 0; i--) {
    total += estimateMessageTokens(byCount[i]);
    if (total > budget) {
      return byCount.slice(i + 1);
    }
  }
  return byCount;
}

// Re-export for consumers that need MessageLike
export type { MessageLike } from "./utils/token-counter";
