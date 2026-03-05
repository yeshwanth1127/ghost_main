/**
 * Chat method handlers (chat.history, chat.send, chat.inject). Our own implementation (ported from Moltbot server-methods/chat).
 */
import { ErrorCodes, errorShape } from "../protocol.js";
import {
  resolveTranscriptPath,
  ensureTranscriptFile,
  readSessionMessages,
  appendUserMessage,
  appendAssistantMessage,
} from "../../session/transcript.js";
import type { MethodRespond, MethodContext } from "./index.js";

export function createChatHandlers(): Record<
  string,
  (params: { params: Record<string, unknown>; respond: MethodRespond; context: MethodContext }) => void | Promise<void>
> {
  return {
    "chat.history": ({ params, respond }) => {
      const sessionKey = typeof params.sessionKey === "string" ? params.sessionKey : undefined;
      if (!sessionKey) {
        console.debug("[ghost-gateway] chat.history: sessionKey required");
        respond(false, undefined, errorShape(ErrorCodes.INVALID_REQUEST, "sessionKey required"));
        return;
      }
      const limit = typeof params.limit === "number" ? params.limit : 200;
      const transcriptPath = resolveTranscriptPath(sessionKey);
      const messages = readSessionMessages(transcriptPath, limit);
      console.debug("[ghost-gateway] chat.history", { sessionKey, limit, messageCount: messages.length });
      respond(true, {
        sessionKey,
        messages,
      });
    },

    "chat.send": ({ params, respond }) => {
      const sessionKey = typeof params.sessionKey === "string" ? params.sessionKey : undefined;
      const message = typeof params.message === "string" ? params.message : undefined;
      const idempotencyKey = typeof params.idempotencyKey === "string" ? params.idempotencyKey : undefined;
      if (!sessionKey || message === undefined) {
        console.debug("[ghost-gateway] chat.send: sessionKey and message required");
        respond(false, undefined, errorShape(ErrorCodes.INVALID_REQUEST, "sessionKey and message required"));
        return;
      }
      const rawMessage = message.trim();
      if (!rawMessage) {
        console.debug("[ghost-gateway] chat.send: message required");
        respond(false, undefined, errorShape(ErrorCodes.INVALID_REQUEST, "message required"));
        return;
      }
      const transcriptPath = resolveTranscriptPath(sessionKey);
      const sessionId = sessionKey.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 200) || "default";
      const ensured = ensureTranscriptFile(transcriptPath, sessionId);
      if (!ensured.ok) {
        console.debug("[ghost-gateway] chat.send: ensure transcript failed", { sessionKey, error: ensured.error });
        respond(false, undefined, errorShape(ErrorCodes.INTERNAL, ensured.error ?? "failed to ensure transcript"));
        return;
      }
      const appended = appendUserMessage({
        transcriptPath,
        sessionId,
        content: rawMessage,
        createIfMissing: true,
      });
      if (!appended.ok) {
        console.debug("[ghost-gateway] chat.send: append failed", { sessionKey, error: appended.error });
        respond(false, undefined, errorShape(ErrorCodes.INTERNAL, appended.error ?? "failed to append message"));
        return;
      }
      const runId = idempotencyKey ?? appended.messageId ?? "run";
      console.debug("[ghost-gateway] chat.send ok", { sessionKey, messageId: appended.messageId, runId });
      respond(true, { runId, status: "started" });
    },

    "chat.inject": ({ params, respond }) => {
      const sessionKey = typeof params.sessionKey === "string" ? params.sessionKey : undefined;
      const message = typeof params.message === "string" ? params.message : undefined;
      const label = typeof params.label === "string" ? params.label : undefined;
      if (!sessionKey || message === undefined) {
        console.debug("[ghost-gateway] chat.inject: sessionKey and message required");
        respond(false, undefined, errorShape(ErrorCodes.INVALID_REQUEST, "sessionKey and message required"));
        return;
      }
      const transcriptPath = resolveTranscriptPath(sessionKey);
      const sessionId = sessionKey.replace(/[^a-zA-Z0-9._-]/g, "_").slice(0, 200) || "default";
      const appended = appendAssistantMessage({
        transcriptPath,
        sessionId,
        message: message.trim(),
        label,
        createIfMissing: true,
      });
      if (!appended.ok) {
        console.debug("[ghost-gateway] chat.inject: append failed", { sessionKey, error: appended.error });
        respond(false, undefined, errorShape(ErrorCodes.INTERNAL, appended.error ?? "failed to inject message"));
        return;
      }
      console.debug("[ghost-gateway] chat.inject ok", { sessionKey, messageId: appended.messageId, label });
      respond(true, { ok: true, messageId: appended.messageId });
    },
  };
}
