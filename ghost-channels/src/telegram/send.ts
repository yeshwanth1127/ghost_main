/**
 * Send a message to Telegram (outbound). Ghost's own implementation.
 */

import type { Api } from "grammy";
import type { Bot } from "grammy";

const MAX_MESSAGE_LENGTH = 4096;

/** Pending permission prompts: ticketId -> resolve(granted). */
const pendingPermissionResolvers = new Map<string, (granted: boolean) => void>();

/** Pending input prompts: chatId -> resolve(inputs). When the user sends the next message, it is used as the input (e.g. path). */
const pendingInputResolvers = new Map<number, (inputs: Record<string, unknown>) => void>();

export type PermissionRequestPayload = {
  requestId?: string;
  runId?: string;
  ticketId?: string;
  humanReadable?: string;
  riskLevel?: string;
  irreversible?: boolean;
};

/**
 * Send a permission prompt to the chat with Allow/Deny buttons.
 * Returns a Promise that resolves with true/false when the user taps.
 */
export function askPermissionInTelegram(
  api: Api,
  chatId: string | number,
  payload: PermissionRequestPayload
): Promise<boolean> {
  const ticketId = payload.ticketId ?? "";
  const text =
    payload.humanReadable?.trim() ||
    "The Pi agent is asking for permission. Allow this action?";
  const escaped = text.replace(/</g, "&lt;").replace(/>/g, "&gt;");

  return new Promise<boolean>((resolve) => {
    pendingPermissionResolvers.set(ticketId, resolve);
    void api
      .sendMessage(chatId, escaped, {
        parse_mode: "HTML",
        reply_markup: {
          inline_keyboard: [
            [
              { text: "✅ Allow", callback_data: `ghost:perm:allow:${ticketId}` },
              { text: "❌ Deny", callback_data: `ghost:perm:deny:${ticketId}` },
            ],
          ],
        },
      })
      .catch(() => {
        pendingPermissionResolvers.delete(ticketId);
        resolve(false);
      });
  });
}

export type InputRequestPayload = {
  inputRequestId?: string;
  runId?: string;
  humanReadable?: string;
  missingFields?: string[];
};

/**
 * Ask the user for input (e.g. file path) in Telegram. Sends a message and returns a Promise that resolves
 * when the user sends their next message in this chat (used as the input value, e.g. { path: text }).
 */
export function askInputInTelegram(
  api: Api,
  chatId: number,
  payload: InputRequestPayload
): Promise<Record<string, unknown>> {
  const text =
    payload.humanReadable?.trim() ||
    (payload.missingFields?.includes("path")
      ? "Enter file path (e.g. /tmp/foo.txt or ~/Documents/name.txt):"
      : "Provide the requested input:");
  const escaped = text.replace(/</g, "&lt;").replace(/>/g, "&gt;");

  return new Promise<Record<string, unknown>>((resolve) => {
    pendingInputResolvers.set(chatId, resolve);
    void api.sendMessage(chatId, escaped, { parse_mode: "HTML" }).catch(() => {
      pendingInputResolvers.delete(chatId);
      resolve({});
    });
  });
}

/**
 * If this chat has a pending input request, resolve it with the given inputs and return true (caller should not dispatch).
 * Otherwise return false.
 */
export function consumePendingInput(chatId: number, text: string): boolean {
  const resolve = pendingInputResolvers.get(chatId);
  if (!resolve) return false;
  pendingInputResolvers.delete(chatId);
  const trimmed = text.trim();
  resolve(trimmed ? { path: trimmed } : {});
  return true;
}

/**
 * Register the callback_query handler for permission buttons.
 * Call once when starting the bot.
 */
export function registerPermissionCallbackHandler(bot: Bot): void {
  bot.on("callback_query:data", async (ctx, next) => {
    const data = ctx.callbackQuery.data;
    if (!data.startsWith("ghost:perm:")) return next();
    const parts = data.split(":");
    if (parts.length < 4) return next();
    const granted = parts[2] === "allow";
    const ticketId = parts.slice(3).join(":"); // in case ticketId contains ':'
    const resolve = pendingPermissionResolvers.get(ticketId);
    if (resolve) {
      pendingPermissionResolvers.delete(ticketId);
      resolve(granted);
    }
    await ctx.answerCallbackQuery();
  });
}

export async function sendToTelegram(
  api: Api,
  chatId: string | number,
  text: string,
  options?: { parse_mode?: "HTML" | "Markdown" }
): Promise<void> {
  if (!text || !text.trim()) return;

  const trimmed = text.trim();
  if (trimmed.length <= MAX_MESSAGE_LENGTH) {
    await api.sendMessage(chatId, trimmed, {
      parse_mode: options?.parse_mode ?? "HTML",
    });
    return;
  }

  for (let i = 0; i < trimmed.length; i += MAX_MESSAGE_LENGTH) {
    const chunk = trimmed.slice(i, i + MAX_MESSAGE_LENGTH);
    await api.sendMessage(chatId, chunk, {
      parse_mode: options?.parse_mode ?? "HTML",
    });
  }
}
