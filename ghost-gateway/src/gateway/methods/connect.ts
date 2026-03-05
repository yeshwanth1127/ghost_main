/**
 * Connect handler. Our own implementation (ported from Moltbot server-methods/connect).
 * First request must be connect; respond with hello-ok.
 */
import { randomUUID } from "node:crypto";
import type { HelloOkPayload } from "../protocol.js";
import { PROTOCOL_VERSION } from "../protocol.js";

export type ConnectContext = {
  connId: string;
  port: number;
  bindHost: string;
};

export type ConnectRespond = (payload: HelloOkPayload) => void;

export function handleConnect(params: {
  connId: string;
  port: number;
  bindHost: string;
  minProtocol: number;
  maxProtocol: number;
  client: { id: string; version: string; platform: string };
}): HelloOkPayload {
  const { connId, port, bindHost, minProtocol, maxProtocol, client } = params;
  console.debug("[ghost-gateway] handleConnect", { connId, clientId: client.id, version: client.version, platform: client.platform });
  const protocol = Math.min(PROTOCOL_VERSION, Math.max(minProtocol, maxProtocol));
  return {
    type: "hello-ok",
    protocol,
    server: {
      version: "1.0.0",
      connId,
      host: bindHost,
    },
    snapshot: {
      presence: [],
      health: { ok: true, ts: Date.now() },
    },
  };
}

