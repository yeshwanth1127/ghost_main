/**
 * Detects when the user wants the AI to look at and respond to visible screen content.
 * When true, we should capture a screenshot and send it to a vision-capable model.
 *
 * Examples:
 * - "Answer the question on my screen"
 * - "What does this say?"
 * - "Help me with this" (when referring to visible content)
 * - "Read what's on my screen"
 */
const SCREEN_CONTENT_PATTERNS = [
  /\banswer\s+(the\s+)?(question|questions)\s+(on\s+)?(my\s+)?screen\b/i,
  /\bquestions?\s+on\s+(my\s+)?screen\b/i,
  /\bwhat('s|s| is)\s+on\s+(my\s+)?screen\b/i,
  /\bwhat\s+does\s+this\s+say\b/i,
  /\bread\s+(what('s|s| is)\s+)?on\s+(my\s+)?screen\b/i,
  /\bhelp\s+me\s+with\s+this\b/i,
  /\bexplain\s+(what('s|s| is)\s+)?on\s+(my\s+)?screen\b/i,
  /\bsolve\s+(this|the\s+question)\b/i,
  /\bwhat('s|s| is)\s+this\s+(question|saying|showing)\b/i,
  /\banswer\s+this\b/i,
  /\brespond\s+to\s+(what('s|s| is)\s+)?on\s+(my\s+)?screen\b/i,
];

/**
 * Returns true if the user message indicates they want the AI to see and respond
 * to content visible on their screen (e.g. a test, document, or app behind Ghost).
 */
export function isScreenContentQuery(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return false;
  return SCREEN_CONTENT_PATTERNS.some((p) => p.test(trimmed));
}
