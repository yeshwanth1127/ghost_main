/**
 * Heuristic: classify message as "task" (Pi agent) vs "conversation" (chat).
 * Matches the desktop taskClassifier so Telegram and desktop route the same way.
 */

const TASK_PATTERNS = [
  /\b(please\s+)?(list|show|get|find|create|write|run|build|make|send|delete|copy|move|add|remove|install|update|check|open|read|search)\b/i,
  /\b(can you|could you|would you)\s+(list|show|get|find|create|write|run|build|make|send|delete|copy|move|add|remove|install|update|check|open|read|search)/i,
  /\b(how to|how do i|steps to)\s+(list|show|get|find|create|write|run|build|make|send|delete|copy|move|add|remove|install|update|check|open|read|search)/i,
  /\b(task|todo|remind me to|schedule|set up)\b/i,
  /^[\w\s]{0,80}\s*[.!]$/,
];

const CONVERSATION_PATTERNS = [
  /^(hi|hello|hey|thanks|thank you|ok|okay|yes|no|bye|goodbye)\s*[.!?]?$/i,
  /\b(what is|what are|who is|when did|where is|why does|how does|explain|define)\b/i,
];

export function isTask(text: string): boolean {
  const t = (text ?? "").trim();
  if (!t || t.length < 2) return false;
  if (CONVERSATION_PATTERNS.some((p) => p.test(t))) return false;
  if (TASK_PATTERNS.some((p) => p.test(t))) return true;
  return false;
}
