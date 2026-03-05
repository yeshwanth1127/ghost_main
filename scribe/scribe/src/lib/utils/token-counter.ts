/**
 * Token counting using js-tiktoken (cl100k_base, GPT-4 compatible).
 * Falls back to char-based estimation if tiktoken fails.
 */
import { getEncoding } from "js-tiktoken";

const CHARS_PER_TOKEN_FALLBACK = 4;

let encoder: ReturnType<typeof getEncoding> | null = null;

function getEncoder() {
  if (!encoder) {
    try {
      encoder = getEncoding("cl100k_base");
    } catch (e) {
      console.warn("[token-counter] Failed to load cl100k_base, using fallback");
      return null;
    }
  }
  return encoder;
}

/**
 * Count tokens in text using tiktoken. Falls back to char/4 if unavailable.
 */
export function countTokens(text: string): number {
  const enc = getEncoder();
  if (enc) {
    try {
      return enc.encode(text || "").length;
    } catch {
      // Fallback on encode error
    }
  }
  return Math.ceil((text || "").length / CHARS_PER_TOKEN_FALLBACK);
}

export interface MessageLike {
  content: string;
  role?: string;
  token_count?: number;
  [key: string]: unknown;
}

/**
 * Estimate tokens for a message (role + content).
 * Uses token_count if present on the message to avoid re-tokenizing.
 */
export function estimateMessageTokens(message: MessageLike): number {
  const cached = (message as { token_count?: number }).token_count;
  if (typeof cached === "number" && cached >= 0) {
    return cached;
  }
  const roleTokens = message.role ? countTokens(message.role) : 0;
  const contentTokens = countTokens(message.content || "");
  return roleTokens + contentTokens;
}
