/**
 * Automatic fact extraction (Layer 3).
 * Extracts stable facts from conversation exchange using AI.
 * Includes fallback regex extraction for common patterns (e.g. name) when AI returns empty.
 */
import { Message } from "@/types";
import { fetchAIResponse } from "./ai-response.function";
import { TYPE_PROVIDER } from "@/types";

const EXTRACTION_PROMPT = `Extract 3-5 important facts from this exchange.
Return JSON only, no markdown. Example format: {"user_name": "Alex", "project_language": "Rust", "database": "PostgreSQL"}
Rules: Only return stable facts. Ignore temporary details. Keys should be snake_case.`;

export interface ExtractFactsProviderOptions {
  provider: TYPE_PROVIDER | undefined;
  selectedProvider: {
    provider: string;
    variables: Record<string, string>;
  };
}

/** Fallback: extract user_name from common patterns when AI returns empty */
function extractNameFromText(text: string): string | null {
  // "my name is Alex" / "my name is John Smith"
  const m1 = text.match(/my name is\s+([A-Za-z][A-Za-z\s'-]{0,50}?)(?:\s+and|\s+\.|\.|,|!|\?|$)/i);
  if (m1) return m1[1].trim();
  // "I'm Alex" - require capitalized (avoids "I'm working")
  const m2 = text.match(/I'm\s+([A-Z][a-z][A-Za-z\s'-]*?)(?:\s|$|\.|,|!|\?)/);
  if (m2) return m2[1].trim();
  // "I am Alex" - require capitalized
  const m3 = text.match(/I am\s+([A-Z][a-z][A-Za-z\s'-]*?)(?:\s|$|\.|,|!|\?)/);
  if (m3) return m3[1].trim();
  // "call me Alex"
  const m4 = text.match(/call me\s+([A-Za-z][A-Za-z\s'-]*?)(?:\s|$|\.|,|!|\?)/i);
  if (m4) return m4[1].trim();
  return null;
}

/**
 * Extract facts from the last N messages.
 */
export async function extractFactsFromExchange(
  messages: Message[],
  options: ExtractFactsProviderOptions
): Promise<Record<string, string>> {
  const conversationText = messages
    .map((m) => {
      const role = m.role === "user" ? "User" : m.role === "assistant" ? "Assistant" : "System";
      const content = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
      return `${role}:\n${content}`;
    })
    .join("\n\n");

  if (!conversationText.trim()) return {};

  let fullResponse = "";
  try {
    for await (const chunk of fetchAIResponse({
      provider: options.provider,
      selectedProvider: options.selectedProvider,
      systemPrompt: EXTRACTION_PROMPT,
      history: [],
      userMessage: `Conversation:\n${conversationText}`,
      imagesBase64: [],
    })) {
      fullResponse += chunk;
    }
  } catch (e) {
    console.warn("[extractFacts] AI extraction failed, trying fallback:", e);
  }

  const trimmed = fullResponse.trim();
  const jsonMatch = trimmed.match(/\{[\s\S]*\}/);
  const jsonStr = jsonMatch ? jsonMatch[0] : trimmed;

  const result: Record<string, string> = {};

  try {
    const parsed = JSON.parse(jsonStr) as Record<string, unknown>;
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof k !== "string" || !k.trim()) continue;
      const val = v == null ? "" : typeof v === "string" ? v : String(v);
      if (val.trim()) result[k.trim()] = val.trim();
    }
  } catch {
    // JSON parse failed, try fallback
  }

  // Fallback: if AI returned nothing useful, extract name from last user message
  if (Object.keys(result).length === 0) {
    const lastUser = [...messages].reverse().find((m) => m.role === "user");
    const userText = typeof lastUser?.content === "string" ? lastUser.content : "";
    const name = extractNameFromText(userText);
    if (name) result.user_name = name;
  }

  return result;
}
