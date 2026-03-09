import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface OllamaCheck {
  ok: boolean;
  configured_url: string;
  configured_model: string;
  available_models: string[];
  error?: string;
}
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";
import { AgentThinkingBox } from "./AgentThinkingBox";
import { AgentLogsPanel } from "./AgentLogsPanel";
import { GatewayChatPanel } from "./GatewayChatPanel";
import { InputRequestDialog } from "./InputRequestDialog";
import { AskUserDialog } from "./AskUserDialog";
import { PermissionRequestDialog } from "./PermissionRequestDialog";
import { isTask } from "./taskClassifier";
import { useGhostGateway } from "@/hooks/useGhostGateway";

interface RunEvent {
  id: number;
  run_id: string;
  event_type: string;
  payload: any;
  created_at: string;
}

interface RunState {
  id: string;
  goal: string;
  status: string;
  messages: any[];
  steps: any[];
  permissions: any[];
  artifacts: any[];
  plan?: {
    id: string;
    goal: string;
    steps: string[];
    summary: string;
    created_at: string;
    revised_at?: string;
  };
}

interface BeliefState {
  known_constraints: string[];
  known_failures: string[];
  current_hypothesis: string | null;
  plan_confidence: number;
  recent_lessons: string[];
}

const GATEWAY_SESSION_KEY = "agent-default";

function isTauriEnv(): boolean {
  if (typeof window === "undefined") return false;
  const w = window as unknown as { __TAURI__?: unknown; __TAURI_INTERNALS__?: unknown };
  return !!(w.__TAURI__ ?? w.__TAURI_INTERNALS__);
}

/** Pi agent is available when Tauri (desktop) or when a Pi API URL is configured (any platform). */
function isPiAvailable(): boolean {
  if (isTauriEnv()) return true;
  // Future: return !!import.meta.env.VITE_PI_API_URL or similar for web/other platforms
  return false;
}

export const AgentView = () => {
  const gateway = useGhostGateway();
  const [sessionKey, setSessionKey] = useState(GATEWAY_SESSION_KEY);
  const [gatewayMessages, setGatewayMessages] = useState<{ role: string; content?: unknown }[]>([]);

  const [runs, setRuns] = useState<string[]>([]);
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const [runState, setRunState] = useState<RunState | null>(null);
  const [events, setEvents] = useState<RunEvent[]>([]);
  const [beliefState, setBeliefState] = useState<BeliefState | null>(null);
  const [pendingInputRequest, setPendingInputRequest] = useState<RunEvent | null>(null);
  const [pendingPermissionRequest, setPendingPermissionRequest] = useState<RunEvent | null>(null);
  const [pendingAskUser, setPendingAskUser] = useState<RunEvent | null>(null);
  const [ollamaCheck, setOllamaCheck] = useState<OllamaCheck | null>(null);
  /** Clear-all flow: null | 'choose' (UI only vs UI+DB) | 'db' (confirm DB delete) */
  const [showClearConfirm, setShowClearConfirm] = useState<null | "choose" | "db">(null);

  const piAvailable = isPiAvailable();
  const { clearDebugLog } = gateway;

  useEffect(() => {
    if (!piAvailable) return;
    let cancelled = false;
    invoke<OllamaCheck>("check_ollama")
      .then((result) => {
        if (!cancelled) setOllamaCheck(result);
      })
      .catch(() => {
        if (!cancelled) setOllamaCheck(null);
      });
    return () => {
      cancelled = true;
    };
  }, [piAvailable]);

  const isRunActive = (status: string) =>
    ["running", "waiting_permission", "waiting_input", "waiting_ask_user", "pending"].includes(status);

  const handleStopRun = async () => {
    if (!selectedRun) return;
    try {
      await invoke("cancel_run", { runId: selectedRun });
      await loadRunState(selectedRun);
    } catch (error) {
      console.error("Failed to stop run:", error);
    }
  };

  const clearUiOnly = useCallback(() => {
    setRuns([]);
    setSelectedRun(null);
    setRunState(null);
    setEvents([]);
    setGatewayMessages([]);
    setPendingInputRequest(null);
    setPendingPermissionRequest(null);
    setPendingAskUser(null);
    setBeliefState(null);
    clearDebugLog();
    setShowClearConfirm(null);
  }, [clearDebugLog]);

  const handleClearAllChoose = useCallback(
    (choice: "ui_only" | "ui_and_db") => {
      if (choice === "ui_only") {
        clearUiOnly();
        return;
      }
      setShowClearConfirm("db");
    },
    [clearUiOnly]
  );

  const handleClearAllDbConfirm = useCallback(async () => {
    try {
      await invoke("clear_all_runs");
      clearUiOnly();
    } catch (e) {
      console.error("clear_all_runs failed", e);
    }
  }, [clearUiOnly]);

  const loadRunState = async (runId: string) => {
    try {
      const state = await invoke<RunState>("get_run_state", { runId });
      setRunState(state);

      const runEvents = await invoke<RunEvent[]>("get_run_events", { runId });
      setEvents(runEvents);

      const inputRequest = runEvents
        .filter((e) => e.event_type === "input.requested")
        .find((e) => {
          const requestId = e.payload.input_request_id;
          return !runEvents.some(
            (ev) => ev.event_type === "input.provided" && ev.payload.input_request_id === requestId
          );
        });

      if (inputRequest) {
        setPendingInputRequest(inputRequest);
      } else {
        setPendingInputRequest(null);
      }

      const permissionRequest = runEvents
        .filter((e) => e.event_type === "permission.requested")
        .find((e) => {
          const permId = e.payload.permission_id ?? e.payload.ticket_id;
          if (!permId) return false;
          return !runEvents.some(
            (ev) =>
              ev.event_type === "permission.decision" &&
              (ev.payload.permission_id === permId || ev.payload.ticket_id === permId)
          );
        });

      if (permissionRequest) {
        setPendingPermissionRequest(permissionRequest);
      } else {
        setPendingPermissionRequest(null);
      }

      const statusLower = (state?.status ?? "").toLowerCase();
      const askUserRequests = runEvents.filter((e) => e.event_type === "ask_user.requested");
      const pendingAsk = statusLower === "waiting_ask_user" && askUserRequests.length > 0
        ? askUserRequests[askUserRequests.length - 1]
        : null;
      if (pendingAsk) {
        setPendingAskUser(pendingAsk);
      } else {
        setPendingAskUser(null);
      }

      try {
        const belief = await invoke<BeliefState>("get_belief_state", { runId });
        setBeliefState(belief);
      } catch {
        setBeliefState(null);
      }
    } catch (error) {
      console.error("Failed to load run state:", error);
    }
  };

  const onSendGoal = useCallback(async (goal: string): Promise<{ ok: boolean; error?: string; runId?: string }> => {
    try {
      const runId = await invoke<string>("create_run", { goal });
      await invoke("start_run", { runId });
      setSelectedRun(runId);
      await loadRunState(runId);
      return { ok: true, runId };
    } catch (e) {
      console.error("Pi agent start failed:", e);
      return { ok: false, error: e instanceof Error ? e.message : String(e) };
    }
  }, []);

  const gatewaySend = useCallback(
    async (text: string): Promise<{ ok: boolean; error?: string; runId?: string }> => {
      const res = await gateway.request("chat.send", { sessionKey, message: text });
      const runId = (res.payload as { runId?: string })?.runId;
      return { ok: res.ok, error: res.error, runId };
    },
    [gateway, sessionKey]
  );

  const onSend = useCallback(
    async (text: string): Promise<{ ok: boolean; error?: string; runId?: string }> => {
      const trimmed = text.trim();
      const task = isTask(trimmed);
      if (import.meta.env.DEV) {
        console.debug("[AgentView] send route", { isTask: task, piAvailable, text: trimmed.slice(0, 60) });
      }
      if (task && piAvailable) return onSendGoal(trimmed);
      return gatewaySend(trimmed);
    },
    [piAvailable, onSendGoal, gatewaySend]
  );

  const loadGatewayHistory = useCallback(async () => {
    const res = await gateway.request("chat.history", { sessionKey, limit: 200 });
    if (!res.ok) return;
    const payload = res.payload as { messages?: { role: string; content?: unknown }[] };
    setGatewayMessages(Array.isArray(payload?.messages) ? payload.messages : []);
  }, [gateway.request, sessionKey]);

  // Subscribe to run events (Tauri only; safe if listen fails or in web)
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<RunEvent>("run_event", (event) => {
      const runEvent = event.payload;
      if (runEvent.run_id === selectedRun && selectedRun) {
        loadRunState(selectedRun);
      }
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (typeof unlisten === "function") unlisten();
    };
  }, [selectedRun]);

  // Poll run state when desktop has an active run so permission/input dialogs appear reliably
  const status = runState?.status ?? "";
  const isActive = ["running", "waiting_permission", "waiting_input", "waiting_ask_user", "pending"].includes(status);
  const shouldPoll = isTauriEnv() && selectedRun && (!runState || isActive);
  useEffect(() => {
    if (!shouldPoll) return;
    const interval = setInterval(() => {
      loadRunState(selectedRun!);
    }, 1500);
    return () => clearInterval(interval);
  }, [selectedRun, shouldPoll]);

  const loadRuns = async () => {
    // Note: Run list functionality will be added when needed
    // Currently, runs are created and selected individually
  };

  // Duplicate loadRunState removed (single definition above); kept as no-op to avoid re-running
  const _noopLoadRunState = async (_unusedRunId: string) => {
    try {
      const state = await invoke<RunState>("get_run_state", { runId });
      setRunState(state);

      const runEvents = await invoke<RunEvent[]>("get_run_events", { runId });
      setEvents(runEvents);

      // Check for pending input requests
      const inputRequest = runEvents
        .filter(e => e.event_type === "input.requested")
        .find(e => {
          // Check if there's no corresponding input.provided event
          const requestId = e.payload.input_request_id;
          return !runEvents.some(
            ev => ev.event_type === "input.provided" && 
            ev.payload.input_request_id === requestId
          );
        });
      
      if (inputRequest) {
        setPendingInputRequest(inputRequest);
      } else {
        setPendingInputRequest(null);
      }

      // Check for pending permission requests
      const permissionRequest = runEvents
        .filter(e => e.event_type === "permission.requested")
        .find(e => {
          // Check if there's no corresponding permission.decision event
          const permId = e.payload.permission_id;
          return !runEvents.some(
            ev => ev.event_type === "permission.decision" && 
            ev.payload.permission_id === permId
          );
        });
      
      if (permissionRequest) {
        setPendingPermissionRequest(permissionRequest);
      } else {
        setPendingPermissionRequest(null);
      }
      
      // Load belief state (cognitive projection)
      try {
        const belief = await invoke<BeliefState>("get_belief_state", { runId });
        setBeliefState(belief);
      } catch (error) {
        console.error("Failed to load belief state:", error);
      }
    } catch (error) {
      console.error("Failed to load run state:", error);
    }
  };

  const handlePermissionReply = async (permissionId: string, granted: boolean) => {
    if (!selectedRun) return;

    try {
      await invoke("reply_permission", {
        runId: selectedRun,
        permissionId,
        granted,
      });
    } catch (error) {
      console.error("Failed to reply to permission:", error);
    }
  };

  const displayMessages: { role: string; content?: unknown }[] =
    selectedRun && runState
      ? [
          { role: "user", content: [{ type: "text", text: runState.goal }] },
          ...(runState.messages ?? []).map((m: { role: string; content: string }) => ({
            role: m.role,
            content: [{ type: "text", text: m.content }],
          })),
        ]
      : gatewayMessages;

  return (
    <div className="w-full h-full flex flex-col gap-4 p-4 min-h-0 overflow-hidden">
      {/* Ollama 404 hint when Pi is available but model unreachable */}
      {piAvailable && ollamaCheck && !ollamaCheck.ok && (
        <div className="rounded-md border border-amber-500/50 bg-amber-50 dark:bg-amber-950/30 px-3 py-2 text-sm text-amber-800 dark:text-amber-200">
          <span className="font-medium">Pi agent needs Ollama: </span>
          {ollamaCheck.error}
          {ollamaCheck.available_models.length > 0 && (
            <span className="block mt-1 text-muted-foreground">
              Set <code className="text-xs bg-muted px-1 rounded">OLLAMA_MODEL</code> in{" "}
              <code className="text-xs bg-muted px-1 rounded">src-tauri/.env</code> to one of:{" "}
              {ollamaCheck.available_models.slice(0, 5).join(", ")}
              {ollamaCheck.available_models.length > 5 && " …"}
            </span>
          )}
        </div>
      )}
      {/* Layout: Chat (left) + Logs panel (right) — Executions, Gateway, Chat logs */}
      <div className="flex-1 min-h-0 flex flex-col lg:flex-row gap-4 overflow-hidden">
        {/* Chat column */}
        <div className="flex-1 min-w-0 min-h-0 flex flex-col">
          {selectedRun && runState && isRunActive(runState.status) && (
            <div className="flex items-center justify-between gap-2 px-2 py-1.5 rounded-md bg-muted/50 border border-border/50 shrink-0 mb-2">
              <span className="text-sm text-muted-foreground truncate">
                Pi run: {runState.status}
              </span>
              <Button
                variant="destructive"
                size="sm"
                onClick={handleStopRun}
                className="shrink-0"
              >
                Stop run
              </Button>
            </div>
          )}
          <GatewayChatPanel
            gateway={gateway}
            sessionKey={sessionKey}
            setSessionKey={setSessionKey}
            messages={displayMessages}
            setMessages={setGatewayMessages}
            loadHistory={loadGatewayHistory}
            onSend={onSend}
            onClearAll={() => setShowClearConfirm("choose")}
            showGatewayDebug={false}
            thinkingBox={
              selectedRun && runState ? (
                <AgentThinkingBox
                  runState={runState}
                  events={events}
                  beliefState={beliefState}
                />
              ) : undefined
            }
          />
        </div>
        {/* Logs panel: Executions | Gateway | Chat */}
        <div className="w-full lg:w-[420px] xl:w-[480px] shrink-0 flex flex-col min-h-[320px]">
          <AgentLogsPanel
            events={events}
            gatewayLog={gateway.debugLog}
            chatLog={displayMessages}
            filterRelevant={true}
          />
        </div>
      </div>

      {/* Input Request Dialog */}
      {pendingInputRequest && selectedRun && (
        <InputRequestDialog
          runId={selectedRun}
          inputRequestId={pendingInputRequest.payload.input_request_id}
          capability={pendingInputRequest.payload.capability || ""}
          intent={pendingInputRequest.payload.intent || ""}
          missingFields={pendingInputRequest.payload.missing_fields || []}
          schema={pendingInputRequest.payload.schema || {}}
          currentInputs={pendingInputRequest.payload.current_inputs || {}}
          onClose={() => {
            setPendingInputRequest(null);
            // Reload state to check for new requests
            if (selectedRun) {
              loadRunState(selectedRun);
            }
          }}
        />
      )}

      {/* Clear all: step 1 – choose UI only or UI + DB */}
      {showClearConfirm === "choose" && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-background border rounded-lg p-6 max-w-md mx-4 shadow-lg">
            <h3 className="text-lg font-semibold mb-2 flex items-center gap-2">
              <Trash2 className="h-5 w-5 text-muted-foreground" />
              Clear chats &amp; debug
            </h3>
            <p className="text-sm text-muted-foreground mb-4">
              Clear the chat list and debug log from this session. Do you also want to permanently delete all run history from the database? This cannot be undone.
            </p>
            <div className="flex flex-wrap justify-end gap-2">
              <Button variant="outline" onClick={() => setShowClearConfirm(null)}>
                Cancel
              </Button>
              <Button variant="secondary" onClick={() => handleClearAllChoose("ui_only")}>
                Clear UI only
              </Button>
              <Button variant="destructive" onClick={() => handleClearAllChoose("ui_and_db")}>
                Clear UI and database
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Clear all: step 2 – confirm DB delete */}
      {showClearConfirm === "db" && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-background border rounded-lg p-6 max-w-md mx-4 shadow-lg">
            <h3 className="text-lg font-semibold mb-2 text-destructive">Delete run history from database?</h3>
            <p className="text-sm text-muted-foreground mb-4">
              This will permanently delete all runs, run events, and execution tickets from the database. This action cannot be undone.
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={() => setShowClearConfirm(null)}>
                No, cancel
              </Button>
              <Button variant="destructive" onClick={handleClearAllDbConfirm}>
                Yes, delete from database
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Permission Request Dialog */}
      {/* Planner asked for clarification (missing details, etc.) */}
      {pendingAskUser && selectedRun && (
        <AskUserDialog
          runId={selectedRun}
          requestId={pendingAskUser.payload.request_id ?? ""}
          question={pendingAskUser.payload.question ?? ""}
          reason={pendingAskUser.payload.reason}
          onClose={() => {
            setPendingAskUser(null);
            if (selectedRun) loadRunState(selectedRun);
          }}
        />
      )}
      {pendingPermissionRequest && selectedRun && (
        <PermissionRequestDialog
          runId={selectedRun}
          permissionId={String(pendingPermissionRequest.payload.permission_id ?? pendingPermissionRequest.payload.ticket_id ?? "").trim()}
          capability={typeof pendingPermissionRequest.payload.capability === "string" ? pendingPermissionRequest.payload.capability : ""}
          scope={typeof pendingPermissionRequest.payload.scope === "string" ? pendingPermissionRequest.payload.scope : JSON.stringify(pendingPermissionRequest.payload.scope ?? "")}
          reason={typeof pendingPermissionRequest.payload.reason === "string" ? pendingPermissionRequest.payload.reason : ""}
          canonicalIntent={pendingPermissionRequest.payload.canonical_intent}
          riskScore={pendingPermissionRequest.payload.risk_score || 0.5}
          onClose={() => {
            setPendingPermissionRequest(null);
            // Reload state to check for new requests
            if (selectedRun) {
              loadRunState(selectedRun);
            }
          }}
        />
      )}
      
      {/* Legacy view (hidden by default, can be toggled) */}
      {false && selectedRun && runState && (
        <Card className="flex-1 p-4 overflow-auto">
          <h3 className="text-md font-semibold mb-2">Run: {runState.goal}</h3>
          <div className="text-sm text-muted-foreground mb-4">
            Status: {runState.status}
          </div>

          {/* Cognitive State (Step 10) */}
          {beliefState && (
            <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <h4 className="font-medium mb-2 text-blue-900 dark:text-blue-100">
                Cognitive State
              </h4>
              <div className="space-y-2 text-sm">
                {beliefState.current_hypothesis && (
                  <div>
                    <span className="font-medium">Hypothesis: </span>
                    <span className="text-muted-foreground">{beliefState.current_hypothesis}</span>
                  </div>
                )}
                <div>
                  <span className="font-medium">Plan Confidence: </span>
                  <span className={`font-mono ${
                    beliefState.plan_confidence > 0.7 ? 'text-green-600 dark:text-green-400' :
                    beliefState.plan_confidence > 0.4 ? 'text-yellow-600 dark:text-yellow-400' :
                    'text-red-600 dark:text-red-400'
                  }`}>
                    {(beliefState.plan_confidence * 100).toFixed(0)}%
                  </span>
                </div>
                {beliefState.known_failures.length > 0 && (
                  <div>
                    <span className="font-medium">Known Failures: </span>
                    <ul className="list-disc list-inside text-muted-foreground mt-1">
                      {beliefState.known_failures.slice(-3).map((failure, idx) => (
                        <li key={idx} className="text-xs">{failure}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {beliefState.recent_lessons.length > 0 && (
                  <div>
                    <span className="font-medium">Lessons Learned: </span>
                    <ul className="list-disc list-inside text-muted-foreground mt-1">
                      {beliefState.recent_lessons.slice(-3).map((lesson, idx) => (
                        <li key={idx} className="text-xs">{lesson}</li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Current Plan */}
          {runState.plan && (
            <div className="mb-4 p-3 bg-primary/10 border border-primary/20 rounded-lg">
              <h4 className="font-medium mb-2 text-primary">
                Current Plan
              </h4>
              <div className="text-sm text-muted-foreground mb-2">
                {runState.plan.summary}
              </div>
              <div className="space-y-1">
                {runState.plan.steps.map((step, idx) => (
                  <div key={idx} className="text-xs flex items-center gap-2">
                    <span className="font-mono text-muted-foreground">
                      {String(idx + 1).padStart(2, "0")}.
                    </span>
                    <span>{step}</span>
                  </div>
                ))}
              </div>
              {runState.plan.revised_at && (
                <div className="text-xs text-muted-foreground mt-2">
                  Revised: {new Date(runState.plan.revised_at).toLocaleTimeString()}
                </div>
              )}
            </div>
          )}

          {/* Latest Artifacts */}
          {runState.artifacts?.length ? (
            <div className="mb-4">
              <h4 className="font-medium mb-2">
                Artifacts ({runState.artifacts.length})
              </h4>
              <div className="space-y-2">
                {runState.artifacts.map((artifact) => (
                  <div
                    key={artifact.id}
                    className="p-2 bg-muted rounded text-sm"
                  >
                    <div className="font-medium">{artifact.summary}</div>
                    <div className="text-xs text-muted-foreground break-all">
                      {artifact.location}
                    </div>
                    {artifact.created_at ? (
                      <div className="text-xs text-muted-foreground">
                        {new Date(artifact.created_at).toLocaleTimeString()}
                      </div>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {/* Messages */}
          {runState.messages?.length ? (
            <div className="mb-4">
              <h4 className="font-medium mb-2">
                Messages ({runState.messages.length})
              </h4>
              <div className="space-y-2">
                {runState.messages.map((message) => (
                  <div key={message.id} className="p-2 bg-muted/60 rounded">
                    <div className="text-xs uppercase text-muted-foreground">
                      {message.role}
                    </div>
                    <div className="text-sm">{message.content}</div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          {/* Pending Permissions */}
          {runState.permissions
            .filter((p) => !p.decision)
            .map((permission) => (
              <div key={permission.id} className="mb-4 p-3 border rounded">
                <p className="font-medium">Permission Request</p>
                <p className="text-sm text-muted-foreground">{permission.reason}</p>
                <div className="flex gap-2 mt-2">
                  <Button
                    size="sm"
                    onClick={() => handlePermissionReply(permission.id, true)}
                  >
                    Allow
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    onClick={() => handlePermissionReply(permission.id, false)}
                  >
                    Deny
                  </Button>
                </div>
              </div>
            ))}

          {/* Steps */}
          <div className="mt-4">
            <h4 className="font-medium mb-2">Planned Steps</h4>
            <div className="space-y-2">
              {runState.steps.length === 0 ? (
                <div className="text-xs text-muted-foreground">
                  No steps yet
                </div>
              ) : (
                runState.steps.map((step, index) => (
                  <div
                    key={step.id}
                    className="flex items-start justify-between gap-3 p-3 bg-muted/60 rounded border border-border/50"
                  >
                    <div className="flex items-start gap-3">
                      <div className="text-xs font-mono text-muted-foreground mt-0.5">
                        {String(index + 1).padStart(2, "0")}
                      </div>
                      <div>
                        <div className="font-medium text-sm">
                          {step.capability || step.tool_name || "Unknown"}
                        </div>
                        {step.intent && (
                          <div className="text-xs text-muted-foreground mt-1">
                            Intent: {step.intent}
                          </div>
                        )}
                        {step.error && (
                          <div className="text-xs text-destructive mt-1">
                            Error: {step.error}
                          </div>
                        )}
                      </div>
                    </div>
                    <div className="text-[11px] uppercase tracking-wide px-2 py-1 rounded bg-background/70 border border-border/60 text-muted-foreground">
                      {step.status}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Events Log */}
          <div className="mt-4">
            <h4 className="font-medium mb-2">Events ({events.length})</h4>
            <div className="space-y-1 min-h-[28rem] max-h-[70vh] overflow-auto">
              {events.map((event) => (
                <div key={event.id} className="text-xs p-1 bg-muted/50 rounded">
                  <span className="font-mono">{event.event_type}</span>
                  <span className="text-muted-foreground ml-2">
                    {new Date(event.created_at).toLocaleTimeString()}
                  </span>
                </div>
              ))}
            </div>
          </div>
        </Card>
      )}
    </div>
  );
};
