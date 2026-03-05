/**
 * WebSocket message handler. Our own implementation (ported from Moltbot ws-connection/message-handler).
 * First frame must be connect; then dispatch requests to method handlers.
 */
import type { WebSocket } from "ws";
import type { ConnectParams, RequestFrame, ResponseFrame } from "./protocol.js";
import { ErrorCodes, errorShape, isConnectParams, isRequestFrame } from "./protocol.js";
import { connectHandlers } from "./methods/index.js";

export type WsContext = {
  connId: string;
  port: number;
  bindHost: string;
  handlers: Record<string, (params: { params: Record<string, unknown>; respond: (ok: boolean, payload?: unknown, error?: { code: string; message: string }) => void; context: { connId: string; port: number; bindHost: string } }) => void | Promise<void>>;
};

export type ClientState = {
  connected: boolean;
};

export function attachWsMessageHandler(params: {
  socket: WebSocket;
  connId: string;
  port: number;
  bindHost: string;
  handlers: WsContext["handlers"];
  getClient: () => ClientState | null;
  setClient: (client: ClientState) => void;
  send: (obj: unknown) => void;
  close: (code?: number, reason?: string) => void;
}): void {
  const { socket, connId, port, bindHost, handlers, getClient, setClient, send, close } = params;
  const context = { connId, port, bindHost };

  socket.on("message", (data: Buffer | ArrayBuffer | Buffer[]) => {
    const raw = Buffer.isBuffer(data) ? data.toString("utf-8") : String(data);
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      console.debug("[ghost-gateway] invalid JSON from client", { connId });
      send({ type: "res", id: "", ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "invalid JSON") });
      close(1008, "invalid JSON");
      return;
    }

    const client = getClient();

      if (!client) {
      if (!isRequestFrame(parsed)) {
        console.debug("[ghost-gateway] handshake failed: first frame must be request", { connId });
        send({ type: "res", id: "", ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "first frame must be request") });
        close(1008, "invalid handshake");
        return;
      }
      const req = parsed as RequestFrame;
      if (req.method !== "connect") {
        console.debug("[ghost-gateway] handshake failed: first request must be connect", { connId, method: req.method });
        send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "first request must be connect") });
        close(1008, "invalid handshake");
        return;
      }
      if (!isConnectParams(req.params)) {
        console.debug("[ghost-gateway] handshake failed: invalid connect params", { connId });
        send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "invalid connect params") });
        close(1008, "invalid handshake");
        return;
      }
      const connectParams = req.params as ConnectParams;
      if (connectParams.maxProtocol < 1 || connectParams.minProtocol > 1) {
        console.debug("[ghost-gateway] handshake failed: protocol mismatch", { connId, min: connectParams.minProtocol, max: connectParams.maxProtocol });
        send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "protocol mismatch") });
        close(1002, "protocol mismatch");
        return;
      }

      console.debug("[ghost-gateway] connect handshake", { connId, clientId: connectParams.client?.id });
      const handler = handlers.connect;
      if (!handler) {
        send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INTERNAL, "connect handler not found") });
        return;
      }
      handler({
        params: req.params as Record<string, unknown>,
        respond: (ok, payload, error) => {
          if (ok) console.debug("[ghost-gateway] connect ok", { connId });
          else console.debug("[ghost-gateway] connect failed", { connId, error: error?.message });
          const res: ResponseFrame = { type: "res", id: req.id, ok };
          if (payload !== undefined) res.payload = payload;
          if (error) res.error = error.message;
          send(res);
        },
        context,
      });
      setClient({ connected: true });
      return;
    }

    if (!isRequestFrame(parsed)) {
      return;
    }
    const req = parsed as RequestFrame;
    if (req.method === "connect") {
      send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, "connect is only valid as the first request") });
      return;
    }

    const handler = handlers[req.method];
    if (!handler) {
      console.debug("[ghost-gateway] unknown method", { connId, method: req.method });
      send({ type: "res", id: req.id, ok: false, error: errorShape(ErrorCodes.INVALID_REQUEST, `unknown method: ${req.method}`) });
      return;
    }
    console.debug("[ghost-gateway] request", { connId, method: req.method, id: req.id });
    handler({
      params: (req.params ?? {}) as Record<string, unknown>,
      respond: (ok, payload, error) => {
        if (!ok) console.debug("[ghost-gateway] response error", { connId, method: req.method, error: error?.message });
        const res: ResponseFrame = { type: "res", id: req.id, ok };
        if (payload !== undefined) res.payload = payload;
        if (error) res.error = error.message;
        send(res);
      },
      context,
    });
  });
}
