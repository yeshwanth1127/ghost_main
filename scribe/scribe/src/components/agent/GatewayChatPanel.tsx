/**
 * Single agent UI: one input for the whole agent (message or goal), gateway connection, message list, debug log.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useGhostGateway } from "@/hooks/useGhostGateway";
import { GHOST_CAPABILITIES_DISPLAY } from "@/config/ghost-capabilities";

type MessageEntry = { role: string; content?: unknown; timestamp?: number };

const DEFAULT_SESSION_KEY = "agent-default";

/** Result of sending a message (either Pi run or gateway chat). */
export type SendResult = { ok: boolean; error?: string; runId?: string };

/** When provided, Send uses this handler; parent can route task → Pi, conversation → Ollama. */
export type OnSend = (text: string) => Promise<SendResult>;

/** Legacy: when set (and onSend not set), Send always uses Pi for the goal. */
export type OnSendGoal = (text: string) => Promise<SendResult>;

export type GatewayChatPanelProps = {
  gateway?: ReturnType<typeof useGhostGateway>;
  sessionKey?: string;
  setSessionKey?: (v: string) => void;
  messages?: MessageEntry[];
  setMessages?: React.Dispatch<React.SetStateAction<MessageEntry[]>>;
  loadHistory?: () => Promise<void>;
  /** Unified send: parent routes task → Pi, conversation → Ollama. Takes precedence over onSendGoal. */
  onSend?: OnSend;
  /** When set (and onSend not set), Send creates/starts a Pi run. */
  onSendGoal?: OnSendGoal;
  /** Optional "AI thinking" / run overview box rendered inside the chat (e.g. Pi agent run state). */
  thinkingBox?: React.ReactNode;
  /** When set, shows a "Clear all" button that calls this (e.g. to clear chats + debug and optionally DB). */
  onClearAll?: () => void;
  /** When false, hide the inline gateway debug section (e.g. when logs are shown in a separate panel). Default true. */
  showGatewayDebug?: boolean;
};

export const GatewayChatPanel = (props: GatewayChatPanelProps) => {
  const internalGateway = useGhostGateway();
  const {
    gateway: propsGateway,
    sessionKey: propsSessionKey,
    setSessionKey: propsSetSessionKey,
    messages: propsMessages,
    setMessages: propsSetMessages,
    loadHistory: propsLoadHistory,
    onSend,
    onSendGoal,
    thinkingBox,
    onClearAll,
    showGatewayDebug = true,
  } = props;

  const gateway = propsGateway ?? internalGateway;
  const {
    status,
    connId,
    debugLog,
    streamState = { runId: null, content: "", thinking: "", error: null },
    remoteRunOverview = null,
    setOnStreamDone = () => {},
    setStreamRunId = () => {},
    connect,
    disconnect,
    request,
  } = gateway;

  const [internalSessionKey, setInternalSessionKey] = useState(DEFAULT_SESSION_KEY);
  const [internalMessages, setInternalMessages] = useState<MessageEntry[]>([]);

  const sessionKey = propsSessionKey ?? internalSessionKey;
  const setSessionKey = propsSetSessionKey ?? setInternalSessionKey;
  const messages = propsMessages ?? internalMessages;
  const setMessages = propsSetMessages ?? setInternalMessages;

  const [inputText, setInputText] = useState("");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sendError, setSendError] = useState<string | null>(null);
  // Default debug open so gateway/agent flow (e.g. @ghost) is visible
  const [debugOpen, setDebugOpen] = useState(true);
  const [thinkingOpen, setThinkingOpen] = useState(true);
  const [showCapabilitiesOpen, setShowCapabilitiesOpen] = useState(false);
  const autoConnectDone = useRef(false);

  const hasDebugErrors = debugLog.some((e) => e.kind === "error");

  const loadHistory = useCallback(async () => {
    setLoadError(null);
    if (propsLoadHistory) {
      await propsLoadHistory();
      return;
    }
    const res = await request("chat.history", { sessionKey, limit: 200 });
    if (!res.ok) {
      setLoadError(res.error ?? "Failed to load history");
      return;
    }
    const payload = res.payload as { sessionKey?: string; messages?: MessageEntry[] };
    setMessages(Array.isArray(payload?.messages) ? payload.messages : []);
  }, [request, sessionKey, setMessages, propsLoadHistory]);

  const loadHistoryRef = useRef(loadHistory);
  loadHistoryRef.current = loadHistory;
  useEffect(() => {
    setOnStreamDone((_messageId: string) => {
      void loadHistoryRef.current();
    });
    return () => setOnStreamDone(null);
  }, [setOnStreamDone]);

  useEffect(() => {
    if (status === "connected") loadHistory();
  }, [status, loadHistory]);

  // Auto-connect once on mount so user can send without clicking Connect
  useEffect(() => {
    if (!autoConnectDone.current && status === "disconnected") {
      autoConnectDone.current = true;
      connect();
    }
  }, [status, connect]);

  const sendMessage = useCallback(async () => {
    const text = inputText.trim();
    if (!text) return;
    setSendError(null);

    const handler = onSend ?? onSendGoal;
    if (handler) {
      const res = await handler(text);
      if (!res.ok) {
        setSendError(res.error ?? "Failed to send");
        return;
      }
      setInputText("");
      setMessages((prev) => [...prev, { role: "user", content: [{ type: "text", text }] }]);
      if (res.runId) setStreamRunId(res.runId);
      return;
    }

    const res = await request("chat.send", { sessionKey, message: text });
    if (!res.ok) {
      setSendError(res.error ?? "Failed to send");
      return;
    }
    setInputText("");
    setMessages((prev) => [...prev, { role: "user", content: [{ type: "text", text }] }]);
    const runId = (res.payload as { runId?: string })?.runId;
    if (runId) setStreamRunId(runId);
  }, [onSend, onSendGoal, request, sessionKey, inputText, setMessages, setStreamRunId]);

  return (
    <Card className="flex flex-col flex-1 min-h-0 p-4 gap-3">
      {/* Compact header: connection + session */}
      <div className="flex flex-wrap items-center justify-between gap-2 shrink-0">
        <div className="flex flex-wrap items-center gap-2">
          <span
            className={`text-sm font-medium px-2 py-1 rounded ${
              status === "connected"
                ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                : status === "connecting"
                  ? "bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400"
                  : "bg-muted text-muted-foreground"
            }`}
          >
            {status === "connected" ? (connId ? `Connected` : "Connected") : status === "connecting" ? "Connecting…" : "Disconnected"}
          </span>
          {status === "connected" ? (
            <Button variant="outline" size="sm" onClick={disconnect}>
              Disconnect
            </Button>
          ) : (
            <Button variant="outline" size="sm" onClick={connect} disabled={status === "connecting"}>
              Connect
            </Button>
          )}
          <label className="text-xs text-muted-foreground flex items-center gap-1">
            Session
            <input
              type="text"
              value={sessionKey}
              onChange={(e) => setSessionKey(e.target.value)}
              className="w-24 px-1.5 py-0.5 text-xs border rounded bg-background"
            />
          </label>
          {status === "connected" && (
            <Button variant="ghost" size="sm" className="text-xs" onClick={loadHistory}>
              Reload history
            </Button>
          )}
        </div>
        {showGatewayDebug && (
          <button
            type="button"
            onClick={() => setDebugOpen((o) => !o)}
            className={`text-xs font-medium px-2 py-1 rounded border transition-colors ${
              hasDebugErrors
                ? "border-destructive/50 text-destructive hover:bg-destructive/10"
                : "text-muted-foreground hover:text-foreground border-transparent hover:border-border"
            }`}
            title="Gateway and agent request/response log"
          >
            {debugOpen ? "▼" : "▶"} Debug ({debugLog.length})
            {hasDebugErrors ? " · errors" : ""}
          </button>
        )}
        {onClearAll && (
          <Button
            variant="ghost"
            size="sm"
            className="text-xs text-muted-foreground hover:text-foreground"
            onClick={onClearAll}
            title="Clear chats and debug log; optionally delete run history from database"
          >
            Clear all
          </Button>
        )}
      </div>
      {loadError && <div className="text-sm text-destructive shrink-0">{loadError}</div>}
      {sendError && <div className="text-sm text-destructive shrink-0">{sendError}</div>}

      {/* Messages: takes remaining space */}
      <div className="flex-1 min-h-0 overflow-auto border rounded-lg p-3 bg-muted/20">
        {messages.length === 0 && !streamState.runId ? (
          <p className="text-sm text-muted-foreground">
            {onSend ?? onSendGoal
              ? "Tasks use the Pi agent (plan, steps, tools). Conversations use Ollama with history."
              : status === "connected"
                ? "No messages yet. Type below to send a message or goal."
                : "Connect to start."}
          </p>
        ) : (
          <div className="space-y-3">
            {messages.map((msg, i) => {
              const content = Array.isArray(msg.content)
                ? (msg.content as { type?: string; text?: string }[]).find((c) => c.type === "text")?.text ?? JSON.stringify(msg.content)
                : typeof msg.content === "string"
                  ? msg.content
                  : JSON.stringify(msg.content);
              return (
                <div
                  key={i}
                  className={`text-sm p-3 rounded-lg ${
                    msg.role === "user" ? "bg-primary/10 ml-8" : "bg-muted/50 mr-8"
                  }`}
                >
                  <span className="text-xs font-medium text-muted-foreground block mb-1">{msg.role}</span>
                  <span className="break-words whitespace-pre-wrap">{content}</span>
                </div>
              );
            })}
            {/* Remote run overview (from @ghost in Telegram/channel) */}
            {remoteRunOverview && (
              <div className="rounded-lg border border-violet-200 dark:border-violet-800 bg-violet-50/50 dark:bg-violet-950/20 p-3 mr-8 space-y-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-xs font-medium text-violet-700 dark:text-violet-400">
                    Remote run (channel)
                  </span>
                  <span
                    className={`text-xs px-2 py-0.5 rounded ${
                      remoteRunOverview.status === "completed"
                        ? "bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-400"
                        : remoteRunOverview.status === "failed" || remoteRunOverview.status === "cancelled"
                          ? "bg-red-100 dark:bg-red-900/40 text-red-700 dark:text-red-400"
                          : remoteRunOverview.status === "waiting_permission"
                            ? "bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-400"
                            : "bg-muted text-muted-foreground"
                    }`}
                  >
                    {remoteRunOverview.status}
                  </span>
                </div>
                <div className="text-sm font-medium text-foreground break-words">
                  {remoteRunOverview.goal}
                </div>
                {remoteRunOverview.pendingPermissionReason && (
                  <p className="text-xs text-muted-foreground">
                    Approve in Telegram: {remoteRunOverview.pendingPermissionReason}
                  </p>
                )}
                {remoteRunOverview.summary && (
                  <pre className="text-xs text-muted-foreground whitespace-pre-wrap break-words font-sans mt-1">
                    {remoteRunOverview.summary}
                  </pre>
                )}
                {remoteRunOverview.error && (
                  <p className="text-xs text-destructive">{remoteRunOverview.error}</p>
                )}
                <p className="text-[10px] text-muted-foreground">
                  Run ID: {remoteRunOverview.runId.slice(0, 8)}…
                </p>
              </div>
            )}
            {/* Pi agent run overview / AI thinking box (inside chat) */}
            {thinkingBox && <div className="mt-2">{thinkingBox}</div>}
            {/* Streaming assistant: content + optional thinking */}
            {streamState.runId && (
              <>
                {streamState.thinking && (
                  <div className="rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50/50 dark:bg-amber-950/20 p-3 mr-8">
                    <button
                      type="button"
                      onClick={() => setThinkingOpen((o) => !o)}
                      className="text-xs font-medium text-amber-700 dark:text-amber-400 mb-1 flex items-center gap-1"
                    >
                      {thinkingOpen ? "▼" : "▶"} Thinking
                    </button>
                    {thinkingOpen && (
                      <pre className="text-xs text-muted-foreground whitespace-pre-wrap break-words font-sans">
                        {streamState.thinking}
                      </pre>
                    )}
                  </div>
                )}
                <div className="text-sm p-3 rounded-lg bg-muted/50 mr-8 border border-primary/20">
                  <span className="text-xs font-medium text-muted-foreground block mb-1">assistant (streaming)</span>
                  {streamState.error ? (
                    <span className="text-destructive">{streamState.error}</span>
                  ) : (
                    <span className="break-words whitespace-pre-wrap">
                      {streamState.content || "…"}
                    </span>
                  )}
                </div>
              </>
            )}
          </div>
        )}
      </div>

      {/* Single input for the whole agent */}
      <div className="flex gap-2 shrink-0 items-end">
        <textarea
          value={inputText}
          onChange={(e) => setInputText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              sendMessage();
            }
          }}
          placeholder={
            onSend ?? onSendGoal
              ? "Message or goal… (task → Pi agent, conversation → Ollama)"
              : status === "connected"
                ? "Message or goal for the agent… (Enter to send)"
                : "Connect first"
          }
          disabled={status !== "connected" && !(onSend ?? onSendGoal)}
          rows={2}
          className="flex-1 min-w-0 px-3 py-2 text-sm border rounded-lg bg-background resize-none focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
        />
        <Button
          variant="outline"
          size="sm"
          onClick={() => setShowCapabilitiesOpen(true)}
          className="self-end shrink-0 whitespace-nowrap"
          title="What can Ghost do?"
        >
          What can I do?
        </Button>
        <Button
          onClick={sendMessage}
          disabled={!inputText.trim() || (status !== "connected" && !(onSend ?? onSendGoal))}
          className="self-end shrink-0"
        >
          Send
        </Button>
      </div>

      <Dialog open={showCapabilitiesOpen} onOpenChange={setShowCapabilitiesOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>What can Ghost do?</DialogTitle>
          </DialogHeader>
          <div className="text-sm text-muted-foreground whitespace-pre-wrap break-words">
            {GHOST_CAPABILITIES_DISPLAY.split(/(\*\*[^*]+\*\*)/g).map((part, i) =>
              part.startsWith("**") && part.endsWith("**") ? (
                <strong key={i} className="font-semibold text-foreground">
                  {part.slice(2, -2)}
                </strong>
              ) : (
                part
              )
            )}
          </div>
        </DialogContent>
      </Dialog>

      {/* Debug log (collapsible): only when showGatewayDebug is true */}
      {showGatewayDebug && debugOpen && (
        <div className="border rounded-lg p-3 flex-1 min-h-[32rem] max-h-[85vh] overflow-auto text-xs font-mono bg-muted/30 space-y-2 flex flex-col">
          <div className="flex items-center justify-between gap-2 mb-1">
            <span className="font-medium text-foreground">Gateway debug</span>
            <span className="text-muted-foreground">Request/response and agent.run flow</span>
          </div>
          {debugLog.length === 0 ? (
            <div className="text-muted-foreground py-2">No entries yet. Connect and send a message or use @ghost from Telegram.</div>
          ) : (
            <ul className="space-y-2 list-none p-0 m-0">
              {debugLog.map((entry, i) => {
                const detailStr =
                  entry.detail == null
                    ? null
                    : typeof entry.detail === "object"
                      ? JSON.stringify(entry.detail, null, 2)
                      : String(entry.detail);
                return (
                  <li
                    key={i}
                    className={`rounded border px-2 py-1.5 ${
                      entry.kind === "error"
                        ? "border-destructive/50 bg-destructive/5"
                        : entry.kind === "connect"
                          ? "border-green-500/30 bg-green-500/5"
                          : entry.kind === "disconnect"
                            ? "border-amber-500/30 bg-amber-500/5"
                            : entry.kind === "request"
                              ? "border-blue-500/30 bg-blue-500/5"
                              : entry.kind === "response"
                                ? "border-emerald-500/30 bg-emerald-500/5"
                                : "border-border bg-muted/20"
                    }`}
                  >
                    <div className="flex flex-wrap items-baseline gap-2">
                      <span className="text-muted-foreground shrink-0 tabular-nums">
                        {new Date(entry.ts).toLocaleTimeString()}
                      </span>
                      <span
                        className={
                          entry.kind === "error"
                            ? "text-destructive font-medium"
                            : entry.kind === "connect"
                              ? "text-green-600 dark:text-green-400"
                              : entry.kind === "disconnect"
                                ? "text-amber-600 dark:text-amber-400"
                                : entry.kind === "request"
                                  ? "text-blue-600 dark:text-blue-400"
                                  : entry.kind === "response"
                                    ? "text-emerald-600 dark:text-emerald-400"
                                    : "text-foreground"
                        }
                      >
                        [{entry.kind}] {entry.message}
                      </span>
                    </div>
                    {detailStr != null && (
                      <pre className="mt-1 mb-0 text-muted-foreground whitespace-pre-wrap break-words font-sans text-[11px] overflow-x-auto">
                        {detailStr}
                      </pre>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      )}
    </Card>
  );
};
