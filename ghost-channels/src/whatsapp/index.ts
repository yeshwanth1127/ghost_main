/**
 * WhatsApp adapter. On message: build context, dispatch to gateway, send reply via sendToWhatsApp.
 */

import makeWASocket, {
  useMultiFileAuthState,
  type WASocket,
} from "@whiskeysockets/baileys";
import type { GatewayClient } from "../gateway-client.js";
import { dispatchInboundMessage } from "../dispatch.js";
import type { InboundContext } from "../types.js";
import { sendToWhatsApp } from "./send.js";

export interface WhatsAppAdapterOptions {
  authDir: string;
  gatewayClient: GatewayClient;
}

export async function startWhatsAppClient(
  options: WhatsAppAdapterOptions
): Promise<WASocket> {
  const { authDir, gatewayClient } = options;
  const { state, saveCreds } = await useMultiFileAuthState(authDir);

  const sock = makeWASocket({
    auth: state,
    printQRInTerminal: true,
  });

  sock.ev.on("creds.update", saveCreds);

  sock.ev.on("messages.upsert", async ({ messages, type }) => {
    if (type !== "notify") return;
    for (const msg of messages) {
      const jid = msg.key?.remoteJid;
      if (!jid) continue;
      if (msg.key?.fromMe) continue;

      const m = msg.message as Record<string, unknown> | undefined;
      const text =
        (m?.conversation as string) ??
        (m?.extendedTextMessage as { text?: string } | undefined)?.text ??
        "";
      const body = String(text ?? "").trim();
      if (!body) continue;

      const participant = msg.key?.participant ?? jid;
      const sessionKey = `whatsapp:${jid}`;
      const inboundContext: InboundContext = {
        Body: body,
        From: `whatsapp:${participant}`,
        To: `whatsapp:${jid}`,
        SessionKey: sessionKey,
      };

      await dispatchInboundMessage(
        inboundContext,
        gatewayClient,
        async (replyText) => {
          await sendToWhatsApp(sock, jid, replyText);
        }
      );
    }
  });

  return sock;
}
