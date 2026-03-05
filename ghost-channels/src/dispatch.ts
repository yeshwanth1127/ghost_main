/**
 * Dispatch inbound message to gateway and send reply via channel outbound.
 * /ghost or @ghost messages go to Pi agent (agent.run).
 * Plain messages that look like tasks (e.g. "create a file") also go to Pi; others go to chat (chat.send).
 */

import type { GatewayClient } from "./gateway-client.js";
import type { InboundContext } from "./types.js";
import { parseGhostCommand } from "./parse-ghost-command.js";
import { isTask } from "./task-classifier.js";

export type OnPermissionRequest = (payload: Record<string, unknown>) => Promise<boolean>;
export type OnInputRequest = (payload: Record<string, unknown>) => Promise<Record<string, unknown>>;

export async function dispatchInboundMessage(
  ctx: InboundContext,
  gatewayClient: GatewayClient,
  sendReply: (text: string) => Promise<void>,
  onPermissionRequest?: OnPermissionRequest,
  onInputRequest?: OnInputRequest
): Promise<void> {
  const body = (ctx.Body ?? "").trim();
  if (!body) return;

  const { isAgent, command } = parseGhostCommand(body);

  try {
    if (isAgent) {
      const cmd = (command ?? "").trim();
      console.log("[ghost-channels] agent.run", { sessionKey: ctx.SessionKey, command: cmd || "(empty)" });
      if (!cmd) {
        await sendReply("Send a command after /ghost or @ghost, e.g. /ghost create a file foo.txt");
        return;
      }
      const result = await gatewayClient.agentRunAndWait(
        ctx.SessionKey,
        cmd,
        onPermissionRequest,
        onInputRequest
      );
      console.log("[ghost-channels] agent.run result", { success: result.success, error: result.error });
      if (result.error) {
        await sendReply(`Error: ${result.error}`);
        return;
      }
      const text = (result.summary ?? "").trim();
      if (text) {
        await sendReply(text);
      } else if (result.success) {
        await sendReply("Done.");
      }
      return;
    }

    // Task-like messages (e.g. "create a file") from Telegram → Pi agent, same as desktop
    if (isTask(body)) {
      console.log("[ghost-channels] task-like message → agent.run", { sessionKey: ctx.SessionKey, body: body.slice(0, 60) });
      const result = await gatewayClient.agentRunAndWait(
        ctx.SessionKey,
        body,
        onPermissionRequest,
        onInputRequest
      );
      if (result.error) {
        await sendReply(`Error: ${result.error}`);
        return;
      }
      const text = (result.summary ?? "").trim();
      if (text) await sendReply(text);
      else if (result.success) await sendReply("Done.");
      return;
    }

    const result = await gatewayClient.chatSendAndWait(ctx.SessionKey, body);
    if (result.error) {
      await sendReply(`Error: ${result.error}`);
      return;
    }
    const content = (result.content ?? "").trim();
    if (content) {
      await sendReply(content);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    await sendReply(`Error: ${message}`).catch(() => {});
  }
}
