/**
 * Summarization trigger logic.
 * When to run background summarization.
 */
import { Message } from "@/types";

const RECENT_HISTORY_TOKEN_THRESHOLD = 6000;
const RECENT_MESSAGES_TO_EXCLUDE = 20;

/**
 * Should we run summarization? (recent history exceeds token threshold)
 */
export function shouldSummarize(
  _history: Message[],
  recentHistoryTokens: number
): boolean {
  return recentHistoryTokens > RECENT_HISTORY_TOKEN_THRESHOLD;
}

/**
 * Get older messages to summarize (exclude last N recent messages).
 */
export function getOldMessagesForSummarization(
  history: Message[]
): Message[] {
  if (history.length <= RECENT_MESSAGES_TO_EXCLUDE) return [];
  return history.slice(0, history.length - RECENT_MESSAGES_TO_EXCLUDE);
}
