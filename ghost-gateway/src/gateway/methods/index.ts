/**
 * Gateway method handlers. Our own implementation (ported from Moltbot server-methods).
 */
import type { ConnectParams } from "../protocol.js";
import { ErrorCodes, errorShape } from "../protocol.js";
import { handleConnect } from "./connect.js";
import { createChatHandlers } from "./chat.js";

export type MethodRespond = (ok: boolean, payload?: unknown, error?: { code: string; message: string }, meta?: Record<string, unknown>) => void;

export type MethodContext = {
  connId: string;
  port: number;
  bindHost: string;
};

export type MethodHandlers = Record<
  string,
  (params: { params: Record<string, unknown>; respond: MethodRespond; context: MethodContext }) => void | Promise<void>
>;

const connectHandlersOnly: MethodHandlers = {
  connect: ({ params, respond, context }) => {
    if (!isConnectParams(params)) {
      respond(false, undefined, errorShape(ErrorCodes.INVALID_REQUEST, "invalid connect params"));
      return;
    }
    const { minProtocol, maxProtocol, client } = params;
    const hello = handleConnect({
      connId: context.connId,
      port: context.port,
      bindHost: context.bindHost,
      minProtocol,
      maxProtocol,
      client: {
        id: client.id,
        version: client.version,
        platform: client.platform,
      },
    });
    respond(true, hello);
  },
};

/** All gateway methods: connect + chat.history, chat.send, chat.inject */
export const gatewayHandlers: MethodHandlers = {
  ...connectHandlersOnly,
  ...createChatHandlers(),
};

/** For WS handshake we only accept connect until connected; then all handlers are available */
export const connectHandlers = connectHandlersOnly;

function isConnectParams(v: unknown): v is ConnectParams {
  if (!v || typeof v !== "object") return false;
  const o = v as Record<string, unknown>;
  if (typeof o.minProtocol !== "number" || typeof o.maxProtocol !== "number") return false;
  if (!o.client || typeof o.client !== "object") return false;
  const c = o.client as Record<string, unknown>;
  return (
    typeof c.id === "string" &&
    typeof c.version === "string" &&
    typeof c.platform === "string"
  );
}
