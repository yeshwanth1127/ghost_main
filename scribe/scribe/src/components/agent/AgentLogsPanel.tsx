/**
 * Logs panel: Executions, Gateway, and Chat in one place.
 * Shows relevant run events, gateway request/response, and chat history as logs.
 */
import { useState } from "react";
import { Card } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Terminal, Wifi, MessageSquare, GitBranch } from "lucide-react";
import type { GatewayDebugEntry } from "@/hooks/useGhostGateway";

interface RunEvent {
  id: number;
  run_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

interface ChatEntry {
  role: string;
  content?: unknown;
}

const RELEVANT_EVENT_TYPES = new Set([
  "run.created",
  "run.status_changed",
  "plan.created",
  "plan.revised",
  "decision.made",
  "router.decision",
  "planner.prompt_sent",
  "step.started",
  "step.evaluated",
  "step.completed",
  "step.failed",
  "permission.requested",
  "input.requested",
  "ask_user.requested",
  "run.evaluated",
  "run.reflected",
]);

function formatEventType(type: string): string {
  return type.replace(/\./g, " · ");
}

/** Payload shape for router.decision events from the backend. */
interface RouterDecisionPayload {
  prompt?: string;
  predicted_intent?: string;
  confidence?: number;
  final_route?: string;
  /** "feature_disabled" | "embedding_unavailable" when routed to LLM with 0% confidence. */
  fallback_reason?: string;
}

function getLatestRouterDecision(events: RunEvent[]): RunEvent | undefined {
  const routerEvents = events.filter((e) => e.event_type === "router.decision");
  return routerEvents.length > 0 ? routerEvents[routerEvents.length - 1] : undefined;
}

export type AgentLogsPanelProps = {
  /** Run events (execution log). */
  events: RunEvent[];
  /** Gateway debug entries (connect, request, response, error). */
  gatewayLog: GatewayDebugEntry[];
  /** Chat messages for session log. */
  chatLog: ChatEntry[];
  /** Optional: filter executions to only "relevant" event types. */
  filterRelevant?: boolean;
};

export const AgentLogsPanel = ({
  events,
  gatewayLog,
  chatLog,
  filterRelevant = true,
}: AgentLogsPanelProps) => {
  const [activeTab, setActiveTab] = useState<string>("router");

  const displayEvents = filterRelevant
    ? events.filter((e) => RELEVANT_EVENT_TYPES.has(e.event_type))
    : events;

  const latestRouter = getLatestRouterDecision(events);
  const routerPayload = latestRouter?.payload as RouterDecisionPayload | undefined;
  const routerCount = events.filter((e) => e.event_type === "router.decision").length;

  return (
    <Card className="flex flex-col flex-1 min-w-0 min-h-[320px] border rounded-lg overflow-hidden">
      <Tabs value={activeTab} onValueChange={setActiveTab} className="flex flex-col flex-1 min-h-0">
        <div className="shrink-0 px-3 pt-3 pb-1 border-b bg-muted/30">
          <TabsList className="grid w-full grid-cols-4">
            <TabsTrigger value="router" className="flex items-center gap-1.5 text-xs">
              <GitBranch className="h-3.5 w-3.5" />
              Router ({routerCount})
            </TabsTrigger>
            <TabsTrigger value="executions" className="flex items-center gap-1.5 text-xs">
              <Terminal className="h-3.5 w-3.5" />
              Executions ({displayEvents.length})
            </TabsTrigger>
            <TabsTrigger value="gateway" className="flex items-center gap-1.5 text-xs">
              <Wifi className="h-3.5 w-3.5" />
              Gateway ({gatewayLog.length})
            </TabsTrigger>
            <TabsTrigger value="chat" className="flex items-center gap-1.5 text-xs">
              <MessageSquare className="h-3.5 w-3.5" />
              Chat ({chatLog.length})
            </TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="router" className="flex-1 min-h-0 m-0 data-[state=inactive]:hidden">
          <ScrollArea className="h-full min-h-[280px]">
            <div className="p-3 space-y-3 text-xs">
              {!latestRouter ? (
                <p className="text-muted-foreground py-4">
                  No router decision yet. Send a goal (task) to start a run — the router will decide: direct (fast) path or LLM planner.
                </p>
              ) : (
                <>
                  <div className="rounded-lg border-2 bg-muted/20 p-3 space-y-2">
                    <h4 className="font-semibold text-foreground">Latest routing decision</h4>
                    <div className="grid gap-1.5 font-mono">
                      <div className="flex flex-wrap gap-2">
                        <span className="text-muted-foreground">Route:</span>
                        <span
                          className={
                            routerPayload?.final_route === "direct"
                              ? "text-green-600 dark:text-green-400 font-medium"
                              : "text-amber-600 dark:text-amber-400 font-medium"
                          }
                        >
                          {routerPayload?.final_route ?? "—"}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <span className="text-muted-foreground">Predicted intent:</span>
                        <span className="text-foreground">
                          {routerPayload?.predicted_intent || "(none)"}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <span className="text-muted-foreground">Confidence:</span>
                        <span className="text-foreground">
                          {routerPayload?.confidence != null
                            ? (routerPayload.confidence * 100).toFixed(1) + "%"
                            : "—"}
                        </span>
                      </div>
                      {routerPayload?.prompt != null && (
                        <div className="flex flex-col gap-1">
                          <span className="text-muted-foreground">Prompt (goal):</span>
                          <span className="text-foreground break-words">{routerPayload.prompt}</span>
                        </div>
                      )}
                    </div>
                  </div>
                  {routerPayload?.final_route === "llm" && (routerPayload?.confidence ?? 0) === 0 && (
                    <div className="rounded-lg border border-amber-500/50 bg-amber-500/10 p-3 text-muted-foreground">
                      <p className="font-medium text-amber-700 dark:text-amber-300 mb-1">
                        Why is it going to the LLM?
                      </p>
                      {routerPayload?.fallback_reason === "feature_disabled" ? (
                        <>
                          <p>
                            The embedding-based router is <strong>not included in this build</strong>. Every goal is sent to the LLM.
                          </p>
                          <p className="mt-2">
                            To enable the fast path: run <strong>npm run tauri:dev:embed</strong> (not <code className="bg-muted px-1 rounded">npm run tauri dev</code>). Then set{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_MODEL_PATH</code> and{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_TOKENIZER_PATH</code> in{" "}
                            <code className="bg-muted px-1 rounded">scribe/src-tauri/.env</code>. See{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_SETUP.md</code> in that folder.
                          </p>
                          <p className="mt-2 text-[11px]">
                            On Windows the <code>embedding-router</code> feature is off by default (linker). Use WSL/Linux/macOS or enable the feature and fix CRT linking if you want the fast path on Windows.
                          </p>
                        </>
                      ) : routerPayload?.fallback_reason === "embedding_unavailable" ? (
                        <>
                          <p>
                            The router is <strong>enabled</strong> but no embedding backend was available for this goal, so it fell back to the LLM.
                          </p>
                          <p className="mt-2">
                            Ensure exactly one of these is set in <code className="bg-muted px-1 rounded">scribe/src-tauri/.env</code>:{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_SERVICE_URL</code> (e.g. <code>http://localhost:8004</code> for the Python service), or ONNX paths (
                            <code className="bg-muted px-1 rounded">EMBEDDING_MODEL_PATH</code>, <code className="bg-muted px-1 rounded">EMBEDDING_TOKENIZER_PATH</code>), or{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_PROVIDER=openai|gemini</code> plus API key. If the Python service was just started, try submitting the goal again.
                          </p>
                        </>
                      ) : (
                        <>
                          <p>
                            The embedding-based router is disabled or unavailable. Every goal is sent to the LLM planner.
                          </p>
                          <p className="mt-2">
                            To enable the direct (fast) path: build with <code className="bg-muted px-1 rounded">--features embedding-router</code> (e.g. <strong>npm run tauri:dev:embed</strong>) and set{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_MODEL_PATH</code> and{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_TOKENIZER_PATH</code> in{" "}
                            <code className="bg-muted px-1 rounded">scribe/src-tauri/.env</code>. See{" "}
                            <code className="bg-muted px-1 rounded">EMBEDDING_SETUP.md</code>.
                          </p>
                        </>
                      )}
                    </div>
                  )}
                  {routerPayload?.final_route === "llm" && (routerPayload?.confidence ?? 0) > 0 && (
                    <div className="rounded border bg-muted/30 p-2 text-muted-foreground">
                      Routed to LLM because confidence ({((routerPayload?.confidence ?? 0) * 100).toFixed(1)}%) is below the fast-path threshold (75%). The planner receives an intent hint.
                    </div>
                  )}
                  {routerPayload?.final_route === "direct" && (
                    <div className="rounded border border-green-500/30 bg-green-500/10 p-2 text-green-700 dark:text-green-300">
                      Fast path: goal was parsed and executed without the LLM.
                    </div>
                  )}
                </>
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="executions" className="flex-1 min-h-0 m-0 data-[state=inactive]:hidden">
          <ScrollArea className="h-full min-h-[280px]">
            <div className="p-3 space-y-2 font-mono text-xs">
              {displayEvents.length === 0 ? (
                <p className="text-muted-foreground py-4">No execution events yet. Start a run to see steps and decisions.</p>
              ) : (
                displayEvents.map((event) => (
                  <div
                    key={event.id}
                    className="rounded border bg-background/80 px-2.5 py-2 space-y-1"
                  >
                    <div className="flex flex-wrap items-baseline gap-2">
                      <span className="text-muted-foreground shrink-0 tabular-nums">
                        {new Date(event.created_at).toLocaleTimeString()}
                      </span>
                      <span className="font-medium text-foreground break-all">
                        {formatEventType(event.event_type)}
                      </span>
                    </div>
                    {event.payload && Object.keys(event.payload).length > 0 && (
                      <details className="mt-1">
                        <summary className="text-muted-foreground cursor-pointer hover:text-foreground">
                          Payload
                        </summary>
                        <pre className="mt-1 text-[10px] text-muted-foreground whitespace-pre-wrap break-words overflow-x-auto">
                          {JSON.stringify(event.payload, null, 2)}
                        </pre>
                      </details>
                    )}
                  </div>
                ))
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="gateway" className="flex-1 min-h-0 m-0 data-[state=inactive]:hidden">
          <ScrollArea className="h-full min-h-[280px]">
            <div className="p-3 space-y-2 font-mono text-xs">
              {gatewayLog.length === 0 ? (
                <p className="text-muted-foreground py-4">No gateway entries yet. Connect and send a message.</p>
              ) : (
                gatewayLog.map((entry, i) => {
                  const detailStr =
                    entry.detail == null
                      ? null
                      : typeof entry.detail === "object"
                        ? JSON.stringify(entry.detail, null, 2)
                        : String(entry.detail);
                  return (
                    <div
                      key={i}
                      className={`rounded border px-2.5 py-2 ${
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
                        <details className="mt-1">
                          <summary className="text-muted-foreground cursor-pointer hover:text-foreground">
                            Detail
                          </summary>
                          <pre className="mt-1 text-[10px] text-muted-foreground whitespace-pre-wrap break-words">
                            {detailStr}
                          </pre>
                        </details>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </ScrollArea>
        </TabsContent>

        <TabsContent value="chat" className="flex-1 min-h-0 m-0 data-[state=inactive]:hidden">
          <ScrollArea className="h-full min-h-[280px]">
            <div className="p-3 space-y-2 text-xs">
              {chatLog.length === 0 ? (
                <p className="text-muted-foreground py-4">No chat messages yet.</p>
              ) : (
                chatLog.map((msg, i) => {
                  const content = Array.isArray(msg.content)
                    ? (msg.content as { type?: string; text?: string }[]).find((c) => c.type === "text")?.text ?? JSON.stringify(msg.content)
                    : typeof msg.content === "string"
                      ? msg.content
                      : JSON.stringify(msg.content);
                  const preview = typeof content === "string" && content.length > 120 ? content.slice(0, 120) + "…" : content;
                  return (
                    <div
                      key={i}
                      className={`rounded border px-2.5 py-2 ${
                        msg.role === "user"
                          ? "border-primary/30 bg-primary/5"
                          : "border-border bg-muted/20"
                      }`}
                    >
                      <span className="font-medium text-muted-foreground">{msg.role}</span>
                      <p className="mt-1 break-words whitespace-pre-wrap">{preview}</p>
                    </div>
                  );
                })
              )}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </Card>
  );
};
