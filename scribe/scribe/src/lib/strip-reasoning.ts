/**
 * Strip model reasoning/thinking from chat content.
 * Some models output internal reasoning before the actual reply - this wastes tokens
 * and clutters the UI. We detect and remove it.
 */

const REASONING_PATTERNS = [
  /^Okay,?\s/i,
  /^said\s+"/i,
  /^I need to respond\s/i,
  /^The user (said|sent|just)\s/i,
  /^Let me (check|make sure|think)\s/i,
  /^I should\s/i,
  /^I'll\s/i,
  /^Since the user\s/i,
  /^That's a\s/i,
  /^They might be\s/i,
  /^My response should\s/i,
  /^I need to\s/i,
  /^Let me make sure\s/i,
  /^No need for\s/i,
  /^No emojis\s/i,
  /^Maybe something like\s/i,
  /^That should work\s/i,
  /^Alright, that'?s\s/i,
  /^Yep, that\s/i,
  /^That sounds good/i,
];

const REPLY_STARTS = /^(Hi|Hello|Hey|Sure|Yes|No|Okay|Alright|Thanks|Got it)[!.]?\s/i;

/** Strip assistantgen-xxx IDs that some APIs append */
function stripAssistantGen(text: string): string {
  return text.replace(/\n?assistantgen-[a-zA-Z0-9_-]+\s*$/g, "").trim();
}

/** Check if text contains reasoning (internal model thinking) */
function hasReasoning(text: string): boolean {
  const first200 = text.slice(0, 200);
  return REASONING_PATTERNS.some((re) => re.test(first200));
}

/**
 * If content contains reasoning and a reply, return only the reply part.
 * Otherwise return the content as-is.
 */
export function stripReasoningFromContent(content: string): string {
  if (!content || content.length < 5) return content;

  let s = stripAssistantGen(content).trim();
  if (!s) return content;

  // Strong signal: assistantgen- means we have the reasoning+reply format
  const hasAssistantGen = /assistantgen-/.test(content);

  // Check for reasoning anywhere in first 300 chars (not just first line)
  const hasReasoningContent = hasReasoning(s) || hasAssistantGen;
  if (!hasReasoningContent) return content;

  const lines = s.split(/\r?\n/);

  // Find the last line that looks like a direct reply (greeting + short)
  let lastReplyIndex = -1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (line.length > 0 && line.length < 250 && REPLY_STARTS.test(line)) {
      lastReplyIndex = i;
    }
  }

  if (lastReplyIndex >= 0) {
    return lines.slice(lastReplyIndex).join("\n").trim();
  }

  // Fallback: last line ending with ? or ! that's short (the actual reply)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (line.length > 0 && line.length < 200 && (line.endsWith("?") || line.endsWith("!"))) {
      return line;
    }
  }

  // Fallback: match "Hello! How can I assist..." anywhere (reply after reasoning)
  const replyMatch = s.match(/((?:Hi|Hello|Hey)[!.]?\s+How can I (?:assist|help) you[^.!?]*[!?])/i);
  if (replyMatch) {
    return replyMatch[1].trim();
  }

  // Reasoning detected but no reply yet - hide until we get it
  return "";
}
