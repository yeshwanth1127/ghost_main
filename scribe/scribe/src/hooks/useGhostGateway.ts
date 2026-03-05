/**
 * Ghost gateway WebSocket client for agent mode.
 * Connects to ghost-gateway, performs connect handshake, and exposes chat.history, chat.send, chat.inject.
 * In Tauri app uses @tauri-apps/plugin-websocket to avoid webview blocking native WebSocket to localhost.
 */
import { useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GHOST_GATEWAY_WS_URL } from "@/config/constants";

export type GatewayStatus = "disconnected" | "connecting" | "connected";

export type GatewayDebugEntry = {
  ts: number;
  kind: "connect" | "disconnect" | "request" | "response" | "error";
  message: string;
  detail?: unknown;
};

/** Server-push stream state (run.chunk / run.think / run.done / run.error). */
export type StreamState = {
  runId: string | null;
  content: string;
  thinking: string;
  error: string | null;
};

/** Overview of the current/last remote run (from @ghost in Telegram/channel). Shown in UI. */
export type RemoteRunOverview = {
  runId: string;
  goal: string;
  status: string;
  summary?: string;
  error?: string;
  pendingPermissionReason?: string;
  startedAt: number;
};

type RequestFrame = { type: "req"; id: string; method: string; params?: Record<string, unknown> };
type ResponseFrame = { type: "res"; id: string; ok: boolean; payload?: unknown; error?: string };
type HelloOkPayload = { type: "hello-ok"; protocol: number; server: { version: string; connId: string; host?: string } };
type RunChunk = { type: "run.chunk"; runId: string; text: string };
type RunThink = { type: "run.think"; runId: string; text: string };
type RunDone = { type: "run.done"; runId: string; messageId: string };
type RunError = { type: "run.error"; runId: string; error: string };

function getConnectParams(): Record<string, unknown> {
  const base = {
    minProtocol: 1,
    maxProtocol: 1,
    client: { id: "scribe", version: "1.0", platform: "web" },
  };
  if (typeof window !== "undefined" && !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return { ...base, role: "operator" };
  }
  return base;
}

const MAX_DEBUG_ENTRIES = 50;

function randomId(): string {
  return Math.random().toString(36).slice(2, 12);
}

/** Tauri WebSocket instance from plugin (send/disconnect/addListener). */
type TauriWs = {
  send(message: string | number[]): Promise<void>;
  disconnect(): Promise<void>;
  addListener(cb: (msg: { type: string; data?: string }) => void): () => void;
};

function isTauriEnv(): boolean {
  return typeof window !== "undefined" && !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

export function useGhostGateway(gatewayUrl: string = GHOST_GATEWAY_WS_URL) {
  const [status, setStatus] = useState<GatewayStatus>("disconnected");
  const [connId, setConnId] = useState<string | null>(null);
  const [debugLog, setDebugLog] = useState<GatewayDebugEntry[]>([]);
  const [streamState, setStreamState] = useState<StreamState>({
    runId: null,
    content: "",
    thinking: "",
    error: null,
  });
  /** Current or last remote run (from @ghost) for UI overview. */
  const [remoteRunOverview, setRemoteRunOverview] = useState<RemoteRunOverview | null>(null);
  const onStreamDoneRef = useRef<((messageId: string) => void) | null>(null);
  const browserWsRef = useRef<WebSocket | null>(null);
  const tauriWsRef = useRef<TauriWs | null>(null);
  const pendingRef = useRef<Map<string, { resolve: (v: { ok: boolean; payload?: unknown; error?: string }) => void }>>(new Map());
  const connectReqIdRef = useRef<string | null>(null);
  const removeTauriListenerRef = useRef<(() => void) | null>(null);
  /** Ticket IDs we already sent tool.permission.requested for (avoid duplicate sends). */
  const sentPermissionTicketsRef = useRef<Set<string>>(new Set());
  /** Input request IDs we already sent tool.input.requested for (avoid duplicate sends). */
  const sentInputRequestIdsRef = useRef<Set<string>>(new Set());

  const addDebug = useCallback((kind: GatewayDebugEntry["kind"], message: string, detail?: unknown) => {
    setDebugLog((prev) => {
      const next = [...prev, { ts: Date.now(), kind, message, detail }];
      return next.slice(-MAX_DEBUG_ENTRIES);
    });
  }, []);

  const sendAgentRunResult = useCallback(
    (requestId: string, success: boolean, summary?: string, error?: string) => {
      const ws = tauriWsRef.current ?? browserWsRef.current;
      if (!ws) {
        console.error("[ghost-gateway] agent.run.result: no WebSocket, cannot send", {
          requestId,
          success,
          error,
        });
        addDebug("error", "agent.run.result no WS", { requestId });
        return;
      }
      const params: Record<string, unknown> = { requestId, success };
      if (summary != null) params.summary = summary;
      if (error != null) params.error = error;
      const payload = {
        type: "req" as const,
        id: randomId(),
        method: "agent.run.result",
        params,
      };
      const send = (ws as { send: (s: string) => void | Promise<void> }).send;
      const p = send.call(ws, JSON.stringify(payload));
      if (p && typeof (p as Promise<void>).catch === "function") {
        (p as Promise<void>).catch((e: unknown) => {
          console.error("[ghost-gateway] agent.run.result send failed", e);
          addDebug("error", "agent.run.result send failed", e);
        });
      }
      addDebug("response", "agent.run.result", { requestId, success });
    },
    [addDebug]
  );

  const runRemoteAgentAndSendResult = useCallback(
    async (requestId: string, message: string) => {
      console.log("[ghost-gateway] agent.run.request start", { requestId, message: message?.slice(0, 80) });
      const startedAt = Date.now();
      let currentRunId: string | undefined;
      try {
        addDebug("request", "agent.run.request", { requestId, goal: message });
        const runId = await invoke<string>("create_run", { goal: message });
        currentRunId = runId;
        setRemoteRunOverview({ runId, goal: message, status: "pending", startedAt });
        console.log("[ghost-gateway] create_run ok", { runId });
        addDebug("request", "create_run", { runId });
        // Timeout so we don't hang forever if Rust start_run blocks (e.g. DB lock)
        const startRunTimeoutMs = 15_000;
        await Promise.race([
          invoke("start_run", { runId }),
          new Promise<never>((_, reject) =>
            setTimeout(() => reject(new Error("start_run timed out (15s)")), startRunTimeoutMs)
          ),
        ]);
        setRemoteRunOverview((prev) => (prev?.runId === runId ? { ...prev, status: "running" } : prev));
        console.log("[ghost-gateway] start_run ok, starting poll loop", { runId });
        addDebug("response", "start_run ok", { runId });
        addDebug("response", "poll loop started (get_run_state every 500ms)", { runId });
        const maxWait = 5 * 60 * 1000;
        const start = Date.now();
        let pollCount = 0;
        sentPermissionTicketsRef.current = new Set();
        sentInputRequestIdsRef.current = new Set();
        while (Date.now() - start < maxWait) {
          await new Promise((r) => setTimeout(r, 500));
          pollCount += 1;
          let state: {
            status: string;
            permissions?: { id: string; reason: string; decision?: unknown }[];
          } | null = null;
          try {
            state = await invoke<{
              status: string;
              goal?: string;
              messages?: { role: string; content: string }[];
              permissions?: { id: string; reason: string; decision?: unknown }[];
            }>("get_run_state", { runId });
          } catch (e) {
            console.error("[ghost-gateway] get_run_state failed", { runId, pollCount, error: e });
            addDebug("error", "get_run_state failed", String(e));
            throw e;
          }
          const statusLower = String(state?.status ?? "").toLowerCase();
          const permCount = state?.permissions?.length ?? 0;
          const pendingPerm = state?.permissions?.find((p) => !p.decision);
          if (pollCount <= 5 || pollCount % 10 === 0) {
            console.log("[ghost-gateway] poll", {
              runId,
              pollCount,
              status: state?.status,
              permissionsLength: permCount,
              pendingPermId: pendingPerm?.id ?? null,
            });
          }
          if (pollCount === 1) {
            addDebug("response", "poll #1", { status: state?.status, permissionsLength: permCount });
          }
          if (pollCount === 3) {
            addDebug("response", "poll #3", { status: state?.status, permissionsLength: permCount });
          }
          if (statusLower === "waiting_permission") {
            console.log("[ghost-gateway] poll waiting_permission", {
              runId,
              permissionsLength: permCount,
              permissions: state?.permissions?.map((p) => ({ id: p.id, reason: p.reason?.slice(0, 40), hasDecision: !!p.decision })),
            });
          }
          setRemoteRunOverview((prev) => {
            if (prev?.runId !== runId) return prev;
            return {
              ...prev,
              status: state?.status ?? prev.status,
              pendingPermissionReason: statusLower === "waiting_permission" ? pendingPerm?.reason : undefined,
            };
          });
          // When waiting for input, forward to gateway so Telegram can ask for path etc.
          if (statusLower === "waiting_input") {
            try {
              const events = await invoke<{ event_type: string; payload: Record<string, unknown> }[]>(
                "get_run_events",
                { runId }
              );
              const inputRequested = events.filter((e) => e.event_type === "input.requested");
              const inputProvided = new Set(
                events.filter((e) => e.event_type === "input.provided").map((e) => e.payload?.input_request_id)
              );
              const pending = inputRequested.find((e) => !inputProvided.has(e.payload?.input_request_id));
              if (pending && pending.payload?.input_request_id) {
                const inputRequestId = pending.payload.input_request_id as string;
                if (!sentInputRequestIdsRef.current.has(inputRequestId)) {
                  sentInputRequestIdsRef.current.add(inputRequestId);
                  const ws = tauriWsRef.current ?? browserWsRef.current;
                  const missingFields = (pending.payload.missing_fields as string[]) ?? [];
                  const humanReadable =
                    missingFields.includes("path")
                      ? "Enter file path (e.g. /tmp/foo.txt or ~/Documents/name.txt):"
                      : "Provide the requested input:";
                  if (ws) {
                    const msg = {
                      type: "req",
                      id: randomId(),
                      method: "tool.input.requested",
                      params: {
                        requestId,
                        runId,
                        inputRequestId,
                        missingFields: pending.payload.missing_fields,
                        schema: pending.payload.schema,
                        currentInputs: pending.payload.current_inputs,
                        humanReadable,
                      },
                    };
                    (ws as { send: (s: string) => void | Promise<void> }).send(JSON.stringify(msg));
                    addDebug("request", "tool.input.requested", { inputRequestId, humanReadable });
                  }
                }
              }
            } catch (e) {
              console.error("[ghost-gateway] get_run_events for waiting_input failed", { runId, error: e });
            }
          }
          // When waiting for permission, forward to gateway so Telegram can show Allow/Deny
          if (statusLower === "waiting_permission" && state?.permissions?.length) {
            const pending = state.permissions.find((p) => !p.decision);
            if (pending && !sentPermissionTicketsRef.current.has(pending.id)) {
              sentPermissionTicketsRef.current.add(pending.id);
              const ws = tauriWsRef.current ?? browserWsRef.current;
              console.log("[ghost-gateway] SENDING tool.permission.requested", {
                requestId,
                runId,
                ticketId: pending.id,
                reason: pending.reason?.slice(0, 60),
                hasWs: !!ws,
              });
              addDebug("request", "tool.permission.requested", {
                ticketId: pending.id,
                reason: pending.reason?.slice(0, 50),
                hasWs: !!ws,
              });
              if (ws) {
                const msg = {
                  type: "req",
                  id: randomId(),
                  method: "tool.permission.requested",
                  params: {
                    requestId,
                    runId,
                    ticketId: pending.id,
                    humanReadable: pending.reason ?? "Allow this action?",
                  },
                };
                (ws as { send: (s: string) => void | Promise<void> }).send(JSON.stringify(msg));
              } else {
                console.warn("[ghost-gateway] NO WebSocket - cannot send tool.permission.requested to gateway");
              }
            }
          }
          const terminal = ["completed", "failed", "cancelled"].includes(statusLower);
          if (terminal) {
            console.log("[ghost-gateway] run terminal", { runId, status: state?.status });
            const lastMsg = state.messages?.filter((m) => m.role === "assistant").pop();
            const summary =
              lastMsg?.content?.slice(0, 500) ?? (state.status === "completed" ? "Done." : state.status);
            const success = statusLower === "completed";
            setRemoteRunOverview((prev) =>
              prev?.runId === runId
                ? { ...prev, status: state?.status ?? prev.status, summary, error: success ? undefined : (state?.status ?? summary) }
                : prev
            );
            sendAgentRunResult(requestId, success, summary, success ? undefined : state?.status);
            return;
          }
        }
        console.warn("[ghost-gateway] run timeout (5 min)", { requestId, runId });
        setRemoteRunOverview((prev) =>
          prev?.runId === runId ? { ...prev, status: "failed", error: "Run timeout (5 min)" } : prev
        );
        sendAgentRunResult(requestId, false, undefined, "Run timeout (5 min)");
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        console.error("[ghost-gateway] agent.run.request failed", err);
        addDebug("error", "agent.run.request failed", error);
        setRemoteRunOverview((prev) =>
          currentRunId && prev?.runId === currentRunId ? { ...prev, status: "failed", error } : prev
        );
        sendAgentRunResult(requestId, false, undefined, error);
      }
    },
    [addDebug, sendAgentRunResult]
  );

  const handleIncoming = useCallback(
    (data: unknown) => {
      const obj = data as Record<string, unknown>;
      const typ = obj?.type as string | undefined;

      // Server-push: run.chunk, run.think, run.done, run.error
      if (typ === "run.chunk") {
        const payload = data as RunChunk;
        setStreamState((s) => {
          const runId = s.runId ?? payload.runId;
          const same = s.runId === payload.runId || runId === payload.runId;
          return same ? { ...s, runId, content: s.content + (payload.text || "") } : s;
        });
        return;
      }
      if (typ === "run.think") {
        const payload = data as RunThink;
        setStreamState((s) => {
          const runId = s.runId ?? payload.runId;
          const same = s.runId === payload.runId || runId === payload.runId;
          return same ? { ...s, runId, thinking: s.thinking + (payload.text || "") } : s;
        });
        return;
      }
      if (typ === "run.done") {
        const payload = data as RunDone;
        const cb = onStreamDoneRef.current;
        if (cb) cb(payload.messageId);
        setStreamState({ runId: null, content: "", thinking: "", error: null });
        addDebug("response", "run.done", { runId: payload.runId, messageId: payload.messageId });
        return;
      }
      if (typ === "run.error") {
        const payload = data as RunError;
        setStreamState((s) => (s.runId === payload.runId ? { ...s, error: payload.error ?? "Unknown error" } : s));
        addDebug("error", "run.error", payload.error);
        return;
      }

      if (typ === "agent.run.request" && isTauriEnv()) {
        const payload = data as Record<string, unknown>;
        const requestId = payload?.requestId as string | undefined;
        const message =
          (payload?.message as string | undefined) ?? (payload?.goal as string | undefined) ?? "";
        if (!requestId || !message) {
          console.error("[ghost-gateway] agent.run.request bad payload", payload);
          addDebug("error", "agent.run.request bad payload", { requestId: !!requestId, hasMessage: !!message });
          return;
        }
        console.log("[ghost-gateway] agent.run.request received from gateway, starting runRemoteAgentAndSendResult", {
          requestId,
          message: message.slice(0, 60),
        });
        void runRemoteAgentAndSendResult(requestId, message);
        return;
      }

      if (typ === "tool.permission.reply" && isTauriEnv()) {
        const payload = data as Record<string, unknown>;
        const runId = (payload?.runId ?? payload?.run_id) as string | undefined;
        const ticketId = (payload?.ticketId ?? payload?.ticket_id) as string | undefined;
        const granted = payload?.granted as boolean | undefined;
        if (runId && ticketId != null) {
          addDebug("response", "tool.permission.reply", { runId, ticketId, granted: !!granted });
          // Use snake_case to match Rust command params (Tauri may not auto-convert)
          void invoke("reply_permission", {
            runId,
            permissionId: ticketId,
            granted: granted ?? false,
          }).catch((e) => {
            console.error("[ghost-gateway] reply_permission failed", e);
            addDebug("error", "reply_permission failed", e);
          });
        } else {
          console.warn("[ghost-gateway] tool.permission.reply missing runId or ticketId", payload);
        }
        return;
      }
      if (typ === "tool.input.reply" && isTauriEnv()) {
        const payload = data as Record<string, unknown>;
        const runId = (payload?.runId ?? payload?.run_id) as string | undefined;
        const inputRequestId = (payload?.inputRequestId ?? payload?.input_request_id) as string | undefined;
        const inputs = payload?.inputs;
        if (runId && inputRequestId != null && inputs != null) {
          addDebug("response", "tool.input.reply", { runId, inputRequestId });
          void invoke("reply_input", {
            runId,
            inputRequestId,
            inputs,
          }).catch((e) => {
            console.error("[ghost-gateway] reply_input failed", e);
            addDebug("error", "reply_input failed", e);
          });
        } else {
          console.warn("[ghost-gateway] tool.input.reply missing runId, inputRequestId, or inputs", payload);
        }
        return;
      }

      const res = data as ResponseFrame;
      if (res.type !== "res") return;

      if (connectReqIdRef.current === res.id) {
        connectReqIdRef.current = null;
        if (res.ok && res.payload && (res.payload as HelloOkPayload).type === "hello-ok") {
          const hello = res.payload as HelloOkPayload;
          setConnId(hello.server?.connId ?? null);
          setStatus("connected");
          addDebug("connect", "Connected", { connId: hello.server?.connId });
        } else {
          setStatus("disconnected");
          addDebug("error", "Connect failed", res.error);
        }
        return;
      }

      const pending = pendingRef.current.get(res.id);
      if (pending) {
        pendingRef.current.delete(res.id);
        pending.resolve({ ok: res.ok, payload: res.payload, error: res.error });
      }
      addDebug("response", res.ok ? "ok" : "error", { id: res.id, ok: res.ok, error: res.error });
    },
    [addDebug, runRemoteAgentAndSendResult]
  );

  const setOnStreamDone = useCallback((cb: ((messageId: string) => void) | null) => {
    onStreamDoneRef.current = cb;
  }, []);

  const setStreamRunId = useCallback((runId: string | null) => {
    setStreamState((s) => (s.runId === runId ? s : { ...s, runId, content: "", thinking: "", error: null }));
  }, []);

  const setDisconnected = useCallback(() => {
    browserWsRef.current = null;
    tauriWsRef.current = null;
    connectReqIdRef.current = null;
    const remove = removeTauriListenerRef.current;
    if (remove) {
      remove();
      removeTauriListenerRef.current = null;
    }
    setStatus("disconnected");
    setConnId(null);
  }, []);

  const connect = useCallback(() => {
    if (isTauriEnv()) {
      // Tauri: use plugin WebSocket so connection to localhost is not blocked
      if (tauriWsRef.current) return;
      setStatus("connecting");
      addDebug("connect", "Connecting to gateway (Tauri WebSocket)...", { url: gatewayUrl });
      const reqId = randomId();
      connectReqIdRef.current = reqId;

      void (async () => {
        try {
          const WebSocketPlugin = await import("@tauri-apps/plugin-websocket");
          const ws = (await WebSocketPlugin.default.connect(gatewayUrl)) as TauriWs;
          tauriWsRef.current = ws;

          const remove = ws.addListener((msg) => {
            if (msg.type === "Text" && typeof msg.data === "string") {
              let data: unknown;
              try {
                data = JSON.parse(msg.data);
              } catch {
                addDebug("error", "Invalid JSON from gateway", msg.data);
                return;
              }
              handleIncoming(data);
            }
            if (msg.type === "Close") {
              setDisconnected();
              addDebug("disconnect", "Connection closed (Tauri)", { reason: msg.data });
            }
          });
          removeTauriListenerRef.current = remove;

          const connectReq: RequestFrame = {
            type: "req",
            id: reqId,
            method: "connect",
            params: getConnectParams(),
          };
          await ws.send(JSON.stringify(connectReq));
          addDebug("request", "connect", { id: reqId });
        } catch (err) {
          setDisconnected();
          addDebug("error", "Tauri WebSocket connect failed", err);
        }
      })();
      return;
    }

    // Browser: native WebSocket
    if (browserWsRef.current?.readyState === WebSocket.OPEN) return;
    setStatus("connecting");
    addDebug("connect", "Connecting to gateway...", { url: gatewayUrl });
    const ws = new WebSocket(gatewayUrl);
    browserWsRef.current = ws;
    const reqId = randomId();
    connectReqIdRef.current = reqId;

    ws.onopen = () => {
      const connectReq: RequestFrame = {
        type: "req",
        id: reqId,
        method: "connect",
        params: getConnectParams(),
      };
      ws.send(JSON.stringify(connectReq));
      addDebug("request", "connect", { id: reqId });
    };

    ws.onmessage = (event) => {
      let data: unknown;
      try {
        data = JSON.parse(typeof event.data === "string" ? event.data : String(event.data));
      } catch {
        addDebug("error", "Invalid JSON from gateway", event.data);
        return;
      }
      handleIncoming(data);
    };

    ws.onerror = () => {
      addDebug("error", "WebSocket error");
    };

    ws.onclose = (ev) => {
      browserWsRef.current = null;
      connectReqIdRef.current = null;
      setStatus("disconnected");
      setConnId(null);
      addDebug("disconnect", "Connection closed", { code: ev.code, reason: ev.reason });
    };
  }, [gatewayUrl, addDebug, handleIncoming, setDisconnected]);

  const disconnect = useCallback(async () => {
    const tauri = tauriWsRef.current;
    const browser = browserWsRef.current;
    if (tauri) {
      try {
        await tauri.disconnect();
      } catch {
        // ignore
      }
      setDisconnected();
      return;
    }
    if (browser) {
      browser.close();
      setDisconnected();
    }
  }, [setDisconnected]);

  const request = useCallback(
    (method: string, params: Record<string, unknown> = {}): Promise<{ ok: boolean; payload?: unknown; error?: string }> => {
      const tauri = tauriWsRef.current;
      const browser = browserWsRef.current;

      if (tauri) {
        if (status !== "connected") return Promise.resolve({ ok: false, error: "Not connected" });
        const id = randomId();
        const frame: RequestFrame = { type: "req", id, method, params };
        addDebug("request", method, { id, params: Object.keys(params) });
        return new Promise((resolve) => {
          const timeout = setTimeout(() => {
            if (pendingRef.current.delete(id)) {
              resolve({ ok: false, error: "Request timeout" });
              addDebug("error", "Request timeout", { id, method });
            }
          }, 15000);
          pendingRef.current.set(id, {
            resolve: (v) => {
              clearTimeout(timeout);
              resolve(v);
            },
          });
          void tauri.send(JSON.stringify(frame));
        });
      }

      if (!browser || browser.readyState !== WebSocket.OPEN) {
        return Promise.resolve({ ok: false, error: "Not connected" });
      }
      const id = randomId();
      const frame: RequestFrame = { type: "req", id, method, params };
      browser.send(JSON.stringify(frame));
      addDebug("request", method, { id, params: Object.keys(params) });
      return new Promise((resolve) => {
        const timeout = setTimeout(() => {
          if (pendingRef.current.delete(id)) {
            resolve({ ok: false, error: "Request timeout" });
            addDebug("error", "Request timeout", { id, method });
          }
        }, 15000);
        pendingRef.current.set(id, {
          resolve: (v) => {
            clearTimeout(timeout);
            resolve(v);
          },
        });
      });
    },
    [addDebug, status]
  );

  const clearDebugLog = useCallback(() => {
    setDebugLog([]);
    setRemoteRunOverview(null);
  }, []);

  return {
    status,
    connId,
    debugLog,
    streamState,
    remoteRunOverview,
    setOnStreamDone,
    setStreamRunId,
    connect,
    disconnect,
    request,
    clearDebugLog,
  };
}
