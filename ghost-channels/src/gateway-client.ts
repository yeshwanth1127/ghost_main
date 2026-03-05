/**
 * WebSocket client to scribe-api gateway. Connect, then chat.send; consume run.chunk / run.done / run.error.
 */

import WebSocket from "ws";

const PROTOCOL_VERSION = 1;

function randomId(): string {
  return Math.random().toString(36).slice(2, 12);
}

export interface RunStreamResult {
  content: string;
  error: string | null;
  done: boolean;
}

export interface AgentRunResult {
  success: boolean;
  summary?: string;
  error?: string;
}

export class GatewayClient {
  private ws: WebSocket | null = null;
  private url: string;
  private connectPromise: Promise<void> | null = null;

  constructor(gatewayUrl: string) {
    this.url = gatewayUrl.replace(/^http:/, "ws:");
  }

  async connect(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) return;
    if (this.connectPromise) return this.connectPromise;

    this.connectPromise = (async (): Promise<void> => {
      return new Promise<void>((resolve, reject) => {
        const ws = new WebSocket(this.url);
        this.ws = ws;

        ws.on("open", () => {
          const id = randomId();
          ws.send(
            JSON.stringify({
              type: "req",
              id,
              method: "connect",
              params: {
                minProtocol: PROTOCOL_VERSION,
                maxProtocol: PROTOCOL_VERSION,
                client: {
                  id: "ghost-channels",
                  version: "1.0.0",
                  platform: "node",
                },
                role: "node",
              },
            })
          );

          const onMessage = (data: WebSocket.RawData) => {
            const text = data.toString();
            let parsed: { type?: string; id?: string; ok?: boolean };
            try {
              parsed = JSON.parse(text) as { type?: string; id?: string; ok?: boolean };
            } catch {
              return;
            }
            if (parsed.type === "res" && parsed.id === id) {
              ws.off("message", onMessage);
              if (parsed.ok) {
                resolve();
              } else {
                reject(new Error("Gateway connect failed"));
              }
            }
          };
          ws.on("message", onMessage);
        });

        ws.on("error", (err) => reject(err));
        ws.on("close", () => {
          this.ws = null;
          this.connectPromise = null;
        });
      });
    })();

    try {
      await this.connectPromise;
    } finally {
      this.connectPromise = null;
    }
  }

  /**
   * Send a message via chat.send and wait for run completion (run.done or run.error).
   * Accumulates run.chunk content; returns when run.done or run.error is received.
   */
  async chatSendAndWait(
    sessionKey: string,
    message: string
  ): Promise<RunStreamResult> {
    await this.connect();
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("Gateway not connected");
    }

    const reqId = randomId();
    const runContent: string[] = [];
    let runError: string | null = null;

    this.ws.send(
      JSON.stringify({
        type: "req",
        id: reqId,
        method: "chat.send",
        params: { sessionKey, message },
      })
    );

    const res = await new Promise<{
      type: string;
      id: string;
      ok: boolean;
      payload?: { runId?: string };
      error?: string;
    }>((resolveRes, rejectRes) => {
      const timeout = setTimeout(() => {
        rejectRes(new Error("Gateway chat.send timeout"));
      }, 15000);
      const handler = (data: WebSocket.RawData) => {
        const text = data.toString();
        try {
          const parsed = JSON.parse(text) as {
            type: string;
            id: string;
            ok: boolean;
            payload?: { runId?: string };
            error?: string;
          };
          if (parsed.type === "res" && parsed.id === reqId) {
            clearTimeout(timeout);
            this.ws!.off("message", handler);
            resolveRes(parsed);
          }
        } catch {
          // ignore
        }
      };
      this.ws!.on("message", handler);
    });

    if (!res.ok) {
      return {
        content: "",
        error: res.error ?? "chat.send failed",
        done: true,
      };
    }

    return new Promise<RunStreamResult>((resolve) => {
      const handler = (data: WebSocket.RawData) => {
        const text = data.toString();
        try {
          const parsed = JSON.parse(text) as {
            type?: string;
            runId?: string;
            text?: string;
            error?: string;
          };
          if (parsed.type === "run.chunk" && parsed.text !== undefined) {
            runContent.push(parsed.text);
          } else if (parsed.type === "run.done") {
            this.ws!.off("message", handler);
            resolve({
              content: runContent.join(""),
              error: null,
              done: true,
            });
          } else if (parsed.type === "run.error") {
            this.ws!.off("message", handler);
            resolve({
              content: runContent.join(""),
              error: parsed.error ?? "Unknown error",
              done: true,
            });
          }
        } catch {
          // ignore
        }
      };
      this.ws!.on("message", handler);
    });
  }

  /**
   * Send a command via agent.run (Pi agent on desktop). Waits for agent.run.result.
   * Use for /ghost or @ghost messages.
   * When the desktop sends tool.permission.requested, calls onPermissionRequest and then sends tool.permission.reply.
   * When the desktop sends tool.input.requested, calls onInputRequest and then sends tool.input.reply with the provided inputs.
   */
  async agentRunAndWait(
    sessionKey: string,
    message: string,
    onPermissionRequest?: (payload: Record<string, unknown>) => Promise<boolean>,
    onInputRequest?: (payload: Record<string, unknown>) => Promise<Record<string, unknown>>
  ): Promise<AgentRunResult> {
    await this.connect();
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return { success: false, error: "Gateway not connected" };
    }

    const requestId = randomId();
    console.log("[ghost-channels] agent.run SEND", { requestId, message: message.trim().slice(0, 60) });
    this.ws.send(
      JSON.stringify({
        type: "req",
        id: requestId,
        method: "agent.run",
        params: { requestId, sessionKey, message: message.trim() },
      })
    );

    return new Promise<AgentRunResult>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.ws?.off("message", handler);
        console.warn("[ghost-channels] agent.run TIMEOUT (5 min)", { requestId });
        reject(new Error("Gateway agent.run timeout (5 min)"));
      }, 5 * 60 * 1000);

      const handler = async (data: WebSocket.RawData) => {
        const text = data.toString();
        try {
          const parsed = JSON.parse(text) as Record<string, unknown> & {
            type?: string;
            id?: string;
            requestId?: string;
            ok?: boolean;
            success?: boolean;
            summary?: string;
            error?: string;
            ticketId?: string;
          };
          if (parsed.type === "tool.permission.requested" || parsed.type === "tool.input.requested" || parsed.type === "agent.run.result" || (parsed.type === "res" && parsed.id === requestId)) {
            console.log("[ghost-channels] agent.run WS message", { type: parsed.type, requestId: parsed.requestId, ticketId: parsed.ticketId, inputRequestId: parsed.inputRequestId });
          }
          if (parsed.type === "tool.input.requested" && onInputRequest) {
            const runId = parsed.runId as string | undefined;
            const inputRequestId = parsed.inputRequestId as string | undefined;
            if (runId && inputRequestId) {
              try {
                const inputs = await onInputRequest(parsed);
                this.ws?.send(
                  JSON.stringify({
                    type: "req",
                    id: randomId(),
                    method: "tool.input.reply",
                    params: { runId, inputRequestId, inputs },
                  })
                );
                console.log("[ghost-channels] sent tool.input.reply", { inputRequestId });
              } catch (e) {
                console.error("[ghost-channels] onInputRequest failed", e);
              }
            } else {
              console.warn("[ghost-channels] tool.input.requested missing runId or inputRequestId", parsed);
            }
            return;
          }
          if (parsed.type === "tool.permission.requested" && onPermissionRequest) {
            const ticketId = parsed.ticketId as string | undefined;
            console.log("[ghost-channels] received tool.permission.requested", { ticketId, humanReadable: (parsed as Record<string, unknown>).humanReadable });
            if (ticketId) {
              const granted = await onPermissionRequest(parsed);
              console.log("[ghost-channels] sending tool.permission.reply", { ticketId, granted });
              this.ws?.send(
                JSON.stringify({
                  type: "req",
                  id: randomId(),
                  method: "tool.permission.reply",
                  params: { ticketId, granted },
                })
              );
            } else {
              console.warn("[ghost-channels] tool.permission.requested missing ticketId", parsed);
            }
            return;
          }
          if (parsed.type === "agent.run.result" && parsed.requestId === requestId) {
            clearTimeout(timeout);
            this.ws!.off("message", handler);
            resolve({
              success: parsed.success ?? false,
              summary: parsed.summary,
              error: parsed.error,
            });
            return;
          }
          if (parsed.type === "res" && parsed.id === requestId && parsed.ok === false) {
            clearTimeout(timeout);
            this.ws!.off("message", handler);
            resolve({
              success: false,
              error: parsed.error ?? "agent.run failed",
            });
          }
        } catch {
          // ignore
        }
      };
      this.ws!.on("message", handler);
    });
  }

  close(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.connectPromise = null;
  }
}
