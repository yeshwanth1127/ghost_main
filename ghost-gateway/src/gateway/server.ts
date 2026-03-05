/**
 * Gateway HTTP + WebSocket server. Our own implementation (ported from Moltbot server-http, ws-connection).
 */
import {
  createServer as createHttpServer,
  type IncomingMessage,
  type Server as HttpServer,
  type ServerResponse,
} from "node:http";
import { randomUUID } from "node:crypto";
import type { WebSocket } from "ws";
import { WebSocketServer } from "ws";
import type { ClientState } from "./ws-handler.js";
import { attachWsMessageHandler } from "./ws-handler.js";
import { gatewayHandlers } from "./methods/index.js";

export type GatewayServer = {
  close: (opts?: { reason?: string }) => Promise<void>;
};

export type StartGatewayOptions = {
  port?: number;
  bind?: string;
};

export async function startGatewayServer(
  port: number = 18789,
  opts: StartGatewayOptions = {},
): Promise<GatewayServer> {
  const bindHost = opts.bind === "lan" ? "0.0.0.0" : "127.0.0.1";
  const clients = new Set<{ state: ClientState }>();

  const httpServer = createHttpServer((req: IncomingMessage, res: ServerResponse) => {
    res.statusCode = 404;
    res.setHeader("Content-Type", "text/plain");
    res.end("Ghost gateway (agent mode). Connect via WebSocket.");
  });

  const wss = new WebSocketServer({ noServer: true });

  httpServer.on("upgrade", (req: IncomingMessage, socket: import("node:stream").Duplex, head: Buffer) => {
    const pathname = req.url?.split("?")[0] ?? "/";
    if (pathname !== "/" && pathname !== "") {
      socket.destroy();
      return;
    }
    wss.handleUpgrade(req, socket, head, (ws: WebSocket) => {
      wss.emit("connection", ws, req);
    });
  });

  wss.on("connection", (socket: WebSocket) => {
    const connId = randomUUID();
    console.debug("[ghost-gateway] WS connection", { connId });
    let client: { state: ClientState } | null = null;
    let closed = false;

    const send = (obj: unknown) => {
      try {
        socket.send(JSON.stringify(obj));
      } catch {
        /* ignore */
      }
    };

    const close = (code = 1000, reason?: string) => {
      if (closed) return;
      closed = true;
      console.debug("[ghost-gateway] WS connection closed", { connId, code, reason });
      if (client) clients.delete(client);
      try {
        socket.close(code, reason);
      } catch {
        /* ignore */
      }
    };

    socket.once("error", () => close());
    socket.once("close", () => close());

    attachWsMessageHandler({
      socket,
      connId,
      port,
      bindHost,
      handlers: gatewayHandlers,
      getClient: () => client?.state ?? null,
      setClient: (state) => {
        client = { state };
        clients.add(client);
      },
      send,
      close,
    });
  });

  await new Promise<void>((resolve, reject) => {
    httpServer.once("error", reject);
    httpServer.listen(port, bindHost, () => {
      httpServer.off("error", reject);
      console.debug("[ghost-gateway] server listening", { port, bindHost });
      resolve();
    });
  });

  return {
    close: async (closeOpts) => {
      return new Promise((resolve) => {
        wss.close(() => {
          httpServer.close(() => resolve());
        });
      });
    },
  };
}
