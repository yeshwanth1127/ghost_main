/**
 * Session transcript (JSONL). Our own implementation (ported from Moltbot session-utils, session-utils.fs).
 */
import fs from "node:fs";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { getGhostStateDir } from "../config/paths.js";

const SESSION_VERSION = 1;

function sanitizeSessionId(sessionKey: string): string {
  return sessionKey.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 200) || "default";
}

export function resolveTranscriptPath(sessionKey: string, sessionsDir?: string): string {
  const dir = sessionsDir ?? path.join(getGhostStateDir(), "sessions");
  const sessionId = sanitizeSessionId(sessionKey);
  return path.join(dir, `${sessionId}.jsonl`);
}

export function ensureTranscriptFile(transcriptPath: string, sessionId: string): { ok: boolean; error?: string } {
  if (fs.existsSync(transcriptPath)) return { ok: true };
  try {
    fs.mkdirSync(path.dirname(transcriptPath), { recursive: true });
    const header = {
      type: "session",
      version: SESSION_VERSION,
      id: sessionId,
      timestamp: new Date().toISOString(),
      cwd: process.cwd(),
    };
    fs.writeFileSync(transcriptPath, `${JSON.stringify(header)}\n`, "utf-8");
    console.debug("[ghost-gateway] transcript created", { transcriptPath, sessionId });
    return { ok: true };
  } catch (err) {
    console.debug("[ghost-gateway] transcript ensure failed", { transcriptPath, error: err instanceof Error ? err.message : String(err) });
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

export function readSessionMessages(
  transcriptPath: string,
  limit: number = 200,
): unknown[] {
  if (!fs.existsSync(transcriptPath)) {
    console.debug("[ghost-gateway] transcript read: file missing", { transcriptPath });
    return [];
  }
  const lines = fs.readFileSync(transcriptPath, "utf-8").split(/\r?\n/);
  const messages: unknown[] = [];
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const parsed = JSON.parse(line) as { type?: string; message?: unknown };
      if (parsed?.type === "message" && parsed?.message) {
        messages.push(parsed.message);
      }
    } catch {
      // ignore bad lines
    }
  }
  const hardMax = 1000;
  const max = Math.min(hardMax, limit);
  if (messages.length <= max) return messages;
  return messages.slice(-max);
}

export function appendUserMessage(params: {
  transcriptPath: string;
  sessionId: string;
  content: string;
  createIfMissing?: boolean;
}): { ok: boolean; messageId?: string; error?: string } {
  const { transcriptPath, sessionId, content, createIfMissing } = params;
  if (!fs.existsSync(transcriptPath)) {
    if (!createIfMissing) return { ok: false, error: "transcript file not found" };
    const ensured = ensureTranscriptFile(transcriptPath, sessionId);
    if (!ensured.ok) return { ok: false, error: ensured.error };
  }
  const messageId = randomUUID().slice(0, 8);
  const now = Date.now();
  const messageBody = {
    role: "user",
    content: [{ type: "text", text: content }],
    timestamp: now,
  };
  const entry = {
    type: "message",
    id: messageId,
    timestamp: new Date(now).toISOString(),
    message: messageBody,
  };
  try {
    fs.appendFileSync(transcriptPath, `${JSON.stringify(entry)}\n`, "utf-8");
  } catch (err) {
    console.debug("[ghost-gateway] append user message failed", { transcriptPath, error: err instanceof Error ? err.message : String(err) });
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
  console.debug("[ghost-gateway] append user message", { transcriptPath, messageId });
  return { ok: true, messageId };
}

export function appendAssistantMessage(params: {
  transcriptPath: string;
  sessionId: string;
  message: string;
  label?: string;
  createIfMissing?: boolean;
}): { ok: boolean; messageId?: string; error?: string } {
  const { transcriptPath, sessionId, message, label, createIfMissing } = params;
  if (!fs.existsSync(transcriptPath)) {
    if (!createIfMissing) return { ok: false, error: "transcript file not found" };
    const ensured = ensureTranscriptFile(transcriptPath, sessionId);
    if (!ensured.ok) return { ok: false, error: ensured.error };
  }
  const messageId = randomUUID().slice(0, 8);
  const now = Date.now();
  const labelPrefix = label ? `[${label}]\n\n` : "";
  const messageBody = {
    role: "assistant",
    content: [{ type: "text", text: `${labelPrefix}${message}` }],
    timestamp: now,
    stopReason: "injected",
    usage: { input: 0, output: 0, totalTokens: 0 },
  };
  const entry = {
    type: "message",
    id: messageId,
    timestamp: new Date(now).toISOString(),
    message: messageBody,
  };
  try {
    fs.appendFileSync(transcriptPath, `${JSON.stringify(entry)}\n`, "utf-8");
  } catch (err) {
    console.debug("[ghost-gateway] append assistant message failed", { transcriptPath, error: err instanceof Error ? err.message : String(err) });
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
  console.debug("[ghost-gateway] append assistant message", { transcriptPath, messageId, label });
  return { ok: true, messageId };
}

export function getSessionsDir(): string {
  return path.join(getGhostStateDir(), "sessions");
}
