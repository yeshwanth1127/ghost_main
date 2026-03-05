/**
 * Send a text message to WhatsApp (outbound). Ghost's own implementation.
 */

import type { WASocket } from "@whiskeysockets/baileys";

export async function sendToWhatsApp(
  sock: WASocket,
  jid: string,
  text: string
): Promise<void> {
  if (!text || !text.trim()) return;
  await sock.sendMessage(jid, { text: text.trim() });
}
