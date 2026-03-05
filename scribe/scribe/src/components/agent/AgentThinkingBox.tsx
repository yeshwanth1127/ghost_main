/**
 * Compact "AI thinking" box shown inside the chat: Run Overview, Execution Timeline,
 * Evaluation Panel, Belief State, Revision State, Event Trace — all in one collapsible box.
 */
import { useMemo, useState } from "react";

interface RunEvent {
  id: number;
  run_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

interface RunState {
  id: string;
  goal: string;
  status: string;
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

export interface AgentThinkingBoxProps {
  runState: RunState;
  events: RunEvent[];
  beliefState: BeliefState | null;
}

interface ExecutionStep {
  step_id: string;
  capability: string;
  intent: string;
  attempts: { success: boolean; reason: string; failure_kind?: string; timestamp: string }[];
}

interface Evaluation {
  step_id: string;
  expected_outcome: string;
  actual_outcome: string;
  success: boolean;
  confidence: number;
  reason: string;
  timestamp: string;
}

interface FailureAnalysis {
  step_id: string;
  failure_kind: string;
  reason: string;
  timestamp: string;
}

function parseExecutionTimeline(events: RunEvent[]): ExecutionStep[] {
  const steps = new Map<string, ExecutionStep>();
  for (const event of events) {
    if (event.event_type === "step.evaluated") {
      const stepId = (event.payload.step_id as string) || `step-${event.id}`;
      const capability = (event.payload.capability as string) || "unknown";
      const intent = (event.payload.intent as string) || "";
      if (!steps.has(stepId)) {
        steps.set(stepId, {
          step_id: stepId,
          capability,
          intent,
          attempts: [],
        });
      }
      const step = steps.get(stepId)!;
      step.attempts.push({
        success: !!event.payload.success,
        reason: (event.payload.reason as string) || "",
        failure_kind: event.payload.failure_kind as string | undefined,
        timestamp: event.created_at,
      });
    }
  }
  return Array.from(steps.values());
}

function parseEvaluations(events: RunEvent[]): Evaluation[] {
  return events
    .filter((e) => e.event_type === "step.evaluated")
    .map((e) => ({
      step_id: (e.payload.step_id as string) || `step-${e.id}`,
      expected_outcome: (e.payload.expected_outcome as string) || "",
      actual_outcome: (e.payload.actual_outcome as string) || "",
      success: !!e.payload.success,
      confidence: (e.payload.confidence as number) || 0,
      reason: (e.payload.reason as string) || "",
      timestamp: e.created_at,
    }));
}

function parseFailureAnalysis(events: RunEvent[]): FailureAnalysis[] {
  return events
    .filter((e) => e.event_type === "failure.analyzed")
    .map((e) => ({
      step_id: (e.payload.step_id as string) || "",
      failure_kind: (e.payload.kind as string) || "Unknown",
      reason: (e.payload.reason as string) || "",
      timestamp: e.created_at,
    }));
}

function parseRevisionState(
  events: RunEvent[],
  beliefState: BeliefState | null
): {
  revision_count: number;
  last_revision_time: string | null;
  consecutive_failures: number;
  cooldown_remaining_seconds: number | null;
  signals_detected: { has_failure: boolean; has_low_confidence: boolean };
} {
  let revisionCount = 0;
  let lastRevisionTime: string | null = null;
  let consecutiveFailures = 0;
  for (const event of events) {
    if (event.event_type === "plan.revised") {
      revisionCount++;
      lastRevisionTime = event.created_at;
      consecutiveFailures = 0;
    } else if (event.event_type === "step.evaluated" && !event.payload.success) {
      consecutiveFailures++;
    }
  }
  let cooldownRemaining: number | null = null;
  if (lastRevisionTime) {
    const elapsed = (Date.now() - new Date(lastRevisionTime).getTime()) / 1000;
    cooldownRemaining = Math.max(0, Math.ceil(5 - elapsed));
  }
  const lastEval = events.filter((e) => e.event_type === "step.evaluated").slice(-1)[0];
  const hasFailure = !!lastEval && !lastEval.payload.success;
  const hasLowConfidence = !!beliefState && beliefState.plan_confidence < 0.3;
  return {
    revision_count: revisionCount,
    last_revision_time: lastRevisionTime,
    consecutive_failures: consecutiveFailures,
    cooldown_remaining_seconds: cooldownRemaining,
    signals_detected: { has_failure: hasFailure, has_low_confidence: hasLowConfidence },
  };
}

function getPhase(events: RunEvent[]): string {
  const last = events[events.length - 1];
  if (!last) return "idle";
  if (last.event_type === "plan.created" || last.event_type === "plan.revised") return "planning";
  if (last.event_type === "step.evaluated") return "executing";
  if (last.event_type === "permission.requested") return "waiting_permission";
  if (last.event_type === "input.requested") return "waiting_input";
  if (last.event_type === "ask_user.requested") return "waiting_ask_user";
  return "idle";
}

export const AgentThinkingBox: React.FC<AgentThinkingBoxProps> = ({
  runState,
  events,
  beliefState,
}) => {
  const [open, setOpen] = useState(true);
  const [eventTraceOpen, setEventTraceOpen] = useState(false);

  const executionTimeline = useMemo(() => parseExecutionTimeline(events), [events]);
  const evaluations = useMemo(() => parseEvaluations(events), [events]);
  const failureAnalysis = useMemo(() => parseFailureAnalysis(events), [events]);
  const revisionInfo = useMemo(() => parseRevisionState(events, beliefState), [events, beliefState]);
  const phase = useMemo(() => getPhase(events), [events]);
  const totalFailures = useMemo(
    () => evaluations.filter((e) => !e.success).length,
    [evaluations]
  );
  const confidence = beliefState?.plan_confidence ?? 0.5;

  return (
    <div className="rounded-lg border border-border/60 bg-muted/30 dark:bg-muted/20 text-sm">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between gap-2 px-3 py-2 text-left hover:bg-muted/40 rounded-t-lg"
      >
        <span className="font-medium text-muted-foreground flex items-center gap-2">
          <span className="text-xs font-medium px-2 py-0.5 rounded-full bg-primary/20 text-primary">
            Pi Agent
          </span>
          Run Overview · {runState.status} · {phase} · {(confidence * 100).toFixed(0)}%
        </span>
        <span className="text-muted-foreground">{open ? "▼" : "▶"}</span>
      </button>
      {open && (
        <div className="px-3 pb-3 space-y-3 border-t border-border/40 pt-2">
          {/* Compact overview row */}
          <div className="grid grid-cols-2 sm:grid-cols-5 gap-2 text-xs">
            <div>
              <span className="text-muted-foreground">Status</span>
              <div className="font-medium">{runState.status}</div>
            </div>
            <div>
              <span className="text-muted-foreground">Phase</span>
              <div className="font-medium">{phase}</div>
            </div>
            <div>
              <span className="text-muted-foreground">Confidence</span>
              <div
                className={`font-mono ${
                  confidence > 0.7
                    ? "text-green-600 dark:text-green-400"
                    : confidence > 0.4
                      ? "text-yellow-600 dark:text-yellow-400"
                      : "text-red-600 dark:text-red-400"
                }`}
              >
                {(confidence * 100).toFixed(0)}%
              </div>
            </div>
            <div>
              <span className="text-muted-foreground">Revisions</span>
              <div className="font-medium">{revisionInfo.revision_count}</div>
            </div>
            <div>
              <span className="text-muted-foreground">Failures</span>
              <div className="font-medium">{totalFailures}</div>
            </div>
          </div>

          {/* Execution Timeline */}
          <div>
            <div className="text-xs font-medium text-muted-foreground mb-1">Execution Timeline</div>
            <div className="space-y-1.5 min-h-48 max-h-[50vh] overflow-auto">
              {executionTimeline.length === 0 ? (
                <div className="text-xs text-muted-foreground">No executions yet</div>
              ) : (
                executionTimeline.map((step, idx) => (
                  <div key={step.step_id} className="border rounded p-2 bg-background/50">
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs font-medium">
                        Step {idx + 1}: {step.capability}
                      </span>
                      <span
                        className={`text-xs px-1.5 py-0.5 rounded ${
                          step.attempts.some((a) => a.success)
                            ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                            : "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400"
                        }`}
                      >
                        {step.attempts.some((a) => a.success) ? "Success" : "Failed"}
                      </span>
                    </div>
                    {step.intent && (
                      <div className="text-xs text-muted-foreground">Intent: {step.intent}</div>
                    )}
                    {step.attempts.map((a, i) => (
                      <div key={i} className="text-xs mt-1 flex items-center gap-2">
                        <span>{a.success ? "✅" : "❌"}</span>
                        <span className="text-muted-foreground">
                          {a.reason || (a.failure_kind ? `${a.failure_kind}` : "—")}
                        </span>
                        <span className="text-muted-foreground">
                          {new Date(a.timestamp).toLocaleTimeString()}
                        </span>
                      </div>
                    ))}
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Evaluation Panel */}
          <div>
            <div className="text-xs font-medium text-muted-foreground mb-1">Evaluation Panel</div>
            <div className="space-y-1.5 min-h-40 max-h-[45vh] overflow-auto">
              {evaluations.length === 0 ? (
                <div className="text-xs text-muted-foreground">No evaluations yet</div>
              ) : (
                evaluations.map((e, idx) => (
                  <div key={idx} className="border rounded p-2 bg-background/50 text-xs">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-mono text-muted-foreground truncate">{e.step_id}</span>
                      <span
                        className={`px-1.5 py-0.5 rounded ${
                          e.success
                            ? "bg-green-100 dark:bg-green-900/30"
                            : "bg-red-100 dark:bg-red-900/30"
                        }`}
                      >
                        {e.success ? "Match" : "Mismatch"}
                      </span>
                    </div>
                    <div className="text-muted-foreground">
                      Expected: {e.expected_outcome} · Actual: {e.actual_outcome} · {(e.confidence * 100).toFixed(0)}% · {e.reason}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>

          {/* Failure Analysis */}
          {failureAnalysis.length > 0 && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">
                Failure Analysis & Recovery
              </div>
              <div className="space-y-1">
                {failureAnalysis.map((f, idx) => (
                  <div key={idx} className="border rounded p-2 bg-red-50/50 dark:bg-red-950/20 text-xs">
                    <span className="font-medium text-red-700 dark:text-red-400">{f.failure_kind}</span>
                    <span className="text-muted-foreground ml-2">{f.step_id}</span>
                    <div className="text-muted-foreground mt-1">{f.reason}</div>
                    <div className="text-muted-foreground">
                      {new Date(f.timestamp).toLocaleString()}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Belief State */}
          {beliefState && (
            <div>
              <div className="text-xs font-medium text-muted-foreground mb-1">Belief State</div>
              <div className="border rounded p-2 bg-background/50 text-xs space-y-1">
                {beliefState.current_hypothesis && (
                  <div>
                    <span className="font-medium">Current Hypothesis: </span>
                    <span className="text-muted-foreground">{beliefState.current_hypothesis}</span>
                  </div>
                )}
                <div>
                  <span className="font-medium">Plan Confidence: </span>
                  <span
                    className={`font-mono ${
                      beliefState.plan_confidence > 0.7
                        ? "text-green-600 dark:text-green-400"
                        : beliefState.plan_confidence > 0.4
                          ? "text-yellow-600 dark:text-yellow-400"
                          : "text-red-600 dark:text-red-400"
                    }`}
                  >
                    {(beliefState.plan_confidence * 100).toFixed(1)}%
                  </span>
                </div>
                {beliefState.known_failures.length > 0 && (
                  <div>
                    <span className="font-medium">Known Failures: </span>
                    <span className="text-muted-foreground">
                      {beliefState.known_failures.join("; ")}
                    </span>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Revision State Inspector */}
          <div>
            <div className="text-xs font-medium text-muted-foreground mb-1">
              Revision State Inspector
            </div>
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs">
              <div>
                <span className="text-muted-foreground">Revision Count</span>
                <div className="font-medium">{revisionInfo.revision_count}</div>
              </div>
              <div>
                <span className="text-muted-foreground">Last Revision</span>
                <div className="font-medium">
                  {revisionInfo.last_revision_time
                    ? new Date(revisionInfo.last_revision_time).toLocaleTimeString()
                    : "Never"}
                </div>
              </div>
              <div>
                <span className="text-muted-foreground">Consecutive Failures</span>
                <div className="font-medium">{revisionInfo.consecutive_failures}</div>
              </div>
              <div>
                <span className="text-muted-foreground">Cooldown Remaining</span>
                <div className="font-medium">
                  {revisionInfo.cooldown_remaining_seconds !== null &&
                  revisionInfo.cooldown_remaining_seconds > 0
                    ? `${revisionInfo.cooldown_remaining_seconds}s`
                    : "N/A"}
                </div>
              </div>
            </div>
            <div className="border rounded p-2 mt-2 bg-muted/30 text-xs">
              <div className="flex items-center gap-2">
                <span>{revisionInfo.signals_detected.has_failure ? "✔" : "✖"}</span>
                <span>Failure detected</span>
              </div>
              <div className="flex items-center gap-2">
                <span>{revisionInfo.signals_detected.has_low_confidence ? "✔" : "✖"}</span>
                <span>Low confidence ({(confidence * 100).toFixed(0)}%)</span>
              </div>
              <div className="flex items-center gap-2">
                <span>
                  {revisionInfo.cooldown_remaining_seconds === null ||
                  revisionInfo.cooldown_remaining_seconds === 0
                    ? "✔"
                    : "✖"}
                </span>
                <span>Cooldown expired</span>
              </div>
              <div className="mt-1 pt-1 border-t font-medium">
                {revisionInfo.signals_detected.has_failure &&
                revisionInfo.signals_detected.has_low_confidence &&
                (revisionInfo.cooldown_remaining_seconds === null ||
                  revisionInfo.cooldown_remaining_seconds === 0)
                  ? "→ Revision would be allowed"
                  : "→ Revision blocked"}
              </div>
            </div>
          </div>

          {/* Event Trace */}
          <div>
            <button
              type="button"
              onClick={() => setEventTraceOpen((o) => !o)}
              className="text-xs font-medium text-muted-foreground hover:text-foreground flex items-center gap-1"
            >
              {eventTraceOpen ? "▼" : "▶"} Event Trace ({events.length})
            </button>
            {eventTraceOpen && (
              <div className="mt-1 space-y-1 min-h-64 max-h-[60vh] overflow-auto text-xs">
                {events.map((event) => (
                  <div key={event.id} className="p-2 bg-muted/50 rounded flex items-center gap-2">
                    <span className="font-mono shrink-0">{event.event_type}</span>
                    <span className="text-muted-foreground shrink-0">
                      {new Date(event.created_at).toLocaleTimeString()}
                    </span>
                    <details className="min-w-0">
                      <summary className="text-muted-foreground cursor-pointer truncate">
                        Payload
                      </summary>
                      <pre className="mt-1 text-[10px] overflow-auto whitespace-pre-wrap break-words">
                        {JSON.stringify(event.payload, null, 2)}
                      </pre>
                    </details>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
