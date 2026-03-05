/**
 * Automatic fact extraction (Layer 3).
 * Extracts stable facts from conversation exchange using AI.
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

  const trimmed = fullResponse.trim();
  const jsonMatch = trimmed.match(/\{[\s\S]*\}/);
  const jsonStr = jsonMatch ? jsonMatch[0] : trimmed;

  try {
    const parsed = JSON.parse(jsonStr) as Record<string, unknown>;
    const result: Record<string, string> = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof k === "string" && typeof v === "string" && k.trim()) {
        result[k.trim()] = v.trim();
      }
    }
    return result;
  } catch {
    return {};
  }
}
