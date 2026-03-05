/**
 * Telegram bot adapter. On message: build context, dispatch to gateway, send reply via sendToTelegram.
 */

import { Bot } from "grammy";
import type { GatewayClient } from "../gateway-client.js";
import { dispatchInboundMessage } from "../dispatch.js";
import type { InboundContext } from "../types.js";
import { sendToTelegram, askPermissionInTelegram, askInputInTelegram, consumePendingInput, registerPermissionCallbackHandler } from "./send.js";

export interface TelegramAdapterOptions {
  botToken: string;
  gatewayClient: GatewayClient;
}

export function startTelegramBot(options: TelegramAdapterOptions): Bot {
  const { botToken, gatewayClient } = options;
  const bot = new Bot(botToken);

  registerPermissionCallbackHandler(bot);

  const sendReply = async (chatId: number, replyText: string) => {
    await sendToTelegram(bot.api, chatId, replyText);
  };

  const onPermissionRequest = (chatId: number) => (payload: Record<string, unknown>) =>
    askPermissionInTelegram(bot.api, chatId, payload as { ticketId?: string; humanReadable?: string });

  const onInputRequest = (chatId: number) => (payload: Record<string, unknown>) =>
    askInputInTelegram(bot.api, chatId, payload as { inputRequestId?: string; humanReadable?: string; missingFields?: string[] });

  bot.on("message:text", async (ctx) => {
    const chatId = ctx.chat.id;
    const fromId = ctx.from?.id ?? 0;
    const text = ctx.message.text?.trim() ?? "";
    if (!text) return;

    if (consumePendingInput(chatId, text)) return;

    const sessionKey = `telegram:${chatId}`;
    const inboundContext: InboundContext = {
      Body: text,
      From: `telegram:user:${fromId}`,
      To: `telegram:chat:${chatId}`,
      SessionKey: sessionKey,
    };

    await dispatchInboundMessage(
      inboundContext,
      gatewayClient,
      (replyText) => sendReply(chatId, replyText),
      onPermissionRequest(chatId),
      onInputRequest(chatId)
    );
  });

  bot.on("message:caption", async (ctx) => {
    const chatId = ctx.chat.id;
    const fromId = ctx.from?.id ?? 0;
    const text = ctx.message.caption?.trim() ?? "";
    if (!text) return;

    if (consumePendingInput(chatId, text)) return;

    const sessionKey = `telegram:${chatId}`;
    const inboundContext: InboundContext = {
      Body: text,
      From: `telegram:user:${fromId}`,
      To: `telegram:chat:${chatId}`,
      SessionKey: sessionKey,
    };

    await dispatchInboundMessage(
      inboundContext,
      gatewayClient,
      (replyText) => sendReply(chatId, replyText),
      onPermissionRequest(chatId),
      onInputRequest(chatId)
    );
  });

  return bot;
}
