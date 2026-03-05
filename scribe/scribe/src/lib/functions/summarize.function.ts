/**
 * Conversation summarization (Layer 2).
 * Uses AI to summarize older messages for context compression.
 */
import { Message } from "@/types";
import { fetchAIResponse } from "./ai-response.function";
import { TYPE_PROVIDER } from "@/types";

const SUMMARIZATION_PROMPT = `Summarize the important facts, decisions, and goals from this conversation segment.
Ignore small talk.
Return a concise summary under 150 words.`;

export interface SummarizationProviderOptions {
  provider: TYPE_PROVIDER | undefined;
  selectedProvider: {
    provider: string;
    variables: Record<string, string>;
  };
}

/**
 * Summarize a conversation segment using AI.
 * Prefer GPT-4o-mini for cost efficiency.
 */
export async function summarizeConversationSegment(
  messages: Message[],
  options: SummarizationProviderOptions
): Promise<string> {
  const conversationText = messages
    .map((m) => {
      const role = m.role === "user" ? "User" : m.role === "assistant" ? "Assistant" : "System";
      const content = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
      return `${role}:\n${content}`;
    })
    .join("\n\n");

  if (!conversationText.trim()) return "";

  let fullResponse = "";
  for await (const chunk of fetchAIResponse({
    provider: options.provider,
    selectedProvider: options.selectedProvider,
    systemPrompt: SUMMARIZATION_PROMPT,
    history: [],
    userMessage: `Conversation:\n${conversationText}`,
    imagesBase64: [],
  })) {
    fullResponse += chunk;
  }
  return fullResponse.trim();
}
