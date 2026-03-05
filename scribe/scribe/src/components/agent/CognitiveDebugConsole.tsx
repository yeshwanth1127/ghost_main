// Cognitive Debug Console
//
// This is a debugging UI, not a user product UI.
// All state is derived from events (event-sourced).
//
// Purpose: Answer "Why did the agent do X?" without logs or breakpoints.

import { useMemo } from "react";
import { Card } from "@/components/ui/card";

// Simple Badge component (if not available)
const Badge: React.FC<{ variant?: "default" | "destructive"; children: React.ReactNode }> = ({ 
  variant = "default", 
  children 
}) => {
  const className = variant === "destructive" 
    ? "px-2 py-1 rounded text-xs bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400"
    : "px-2 py-1 rounded text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400";
  return <span className={className}>{children}</span>;
};

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

interface CognitiveDebugConsoleProps {
  runState: RunState;
  events: RunEvent[];
  beliefState: BeliefState | null;
}

// Parse events to extract execution timeline
interface ExecutionStep {
  step_id: string;
  capability: string;
  intent: string;
  inputs: any;
  attempts: ExecutionAttempt[];
  first_attempt_time: string;
  last_attempt_time: string;
}

interface ExecutionAttempt {
  attempt_number: number;
  retry_count: number;
  success: boolean;
  confidence: number;
  reason: string;
  failure_kind?: string;
  timestamp: string;
}

function parseExecutionTimeline(events: RunEvent[]): ExecutionStep[] {
  const steps = new Map<string, ExecutionStep>();
  
  for (const event of events) {
    if (event.event_type === "step.evaluated") {
      const stepId = event.payload.step_id || `step-${event.id}`;
      const capability = event.payload.capability || "unknown";
      const intent = event.payload.intent || "";
      const retryCount = event.payload.retry_count || 0;
      
      if (!steps.has(stepId)) {
        steps.set(stepId, {
          step_id: stepId,
          capability,
          intent,
          inputs: event.payload.inputs || {},
          attempts: [],
          first_attempt_time: event.created_at,
          last_attempt_time: event.created_at,
        });
      }
      
      const step = steps.get(stepId)!;
      step.last_attempt_time = event.created_at;
      
      step.attempts.push({
        attempt_number: step.attempts.length + 1,
        retry_count: retryCount,
        success: event.payload.success || false,
        confidence: event.payload.confidence || 0,
        reason: event.payload.reason || "",
        failure_kind: event.payload.failure_kind,
        timestamp: event.created_at,
      });
    }
  }
  
  return Array.from(steps.values());
}

// Parse evaluation events
interface Evaluation {
  step_id: string;
  expected_outcome: string;
  actual_outcome: string;
  success: boolean;
  confidence: number;
  reason: string;
  timestamp: string;
}

function parseEvaluations(events: RunEvent[]): Evaluation[] {
  return events
    .filter(e => e.event_type === "step.evaluated")
    .map(e => ({
      step_id: e.payload.step_id || `step-${e.id}`,
      expected_outcome: e.payload.expected_outcome || "",
      actual_outcome: e.payload.actual_outcome || "",
      success: e.payload.success || false,
      confidence: e.payload.confidence || 0,
      reason: e.payload.reason || "",
      timestamp: e.created_at,
    }));
}

// Parse failure analysis
interface FailureAnalysis {
  step_id: string;
  failure_kind: string;
  reason: string;
  recovery_action?: string;
  revision_blocked?: boolean;
  timestamp: string;
}

function parseFailureAnalysis(events: RunEvent[]): FailureAnalysis[] {
  const failures: FailureAnalysis[] = [];
  
  for (const event of events) {
    if (event.event_type === "failure.analyzed") {
      failures.push({
        step_id: event.payload.step_id || `step-${event.id}`,
        failure_kind: event.payload.kind || "Unknown",
        reason: event.payload.reason || "",
        timestamp: event.created_at,
      });
    }
  }
  
  return failures;
}

// Parse revision state
interface RevisionInfo {
  revision_count: number;
  last_revision_time: string | null;
  last_revision_reason: string | null;
  consecutive_failures: number;
  cooldown_remaining_seconds: number | null;
  signals_detected: {
    has_failure: boolean;
    has_low_confidence: boolean;
  };
}

function parseRevisionState(
  events: RunEvent[],
  beliefState: BeliefState | null
): RevisionInfo {
  let revisionCount = 0;
  let lastRevisionTime: string | null = null;
  let lastRevisionReason: string | null = null;
  let consecutiveFailures = 0;
  
  for (const event of events) {
    if (event.event_type === "plan.revised") {
      revisionCount++;
      lastRevisionTime = event.created_at;
      lastRevisionReason = event.payload.reason || null;
      consecutiveFailures = 0; // Reset on revision
    } else if (event.event_type === "step.evaluated") {
      if (!event.payload.success) {
        consecutiveFailures++;
      } else {
        consecutiveFailures = 0;
      }
    }
  }
  
  // Calculate cooldown remaining (5 seconds cooldown)
  let cooldownRemaining: number | null = null;
  if (lastRevisionTime) {
    const lastRevision = new Date(lastRevisionTime).getTime();
    const now = Date.now();
    const elapsed = (now - lastRevision) / 1000; // seconds
    const cooldownSeconds = 5;
    const remaining = cooldownSeconds - elapsed;
    cooldownRemaining = remaining > 0 ? Math.ceil(remaining) : 0;
  }
  
  // Check signals
  const lastEvaluation = events
    .filter(e => e.event_type === "step.evaluated")
    .slice(-1)[0];
  const hasFailure = lastEvaluation && !lastEvaluation.payload.success;
  const hasLowConfidence = beliefState && beliefState.plan_confidence < 0.3;
  
  return {
    revision_count: revisionCount,
    last_revision_time: lastRevisionTime,
    last_revision_reason: lastRevisionReason,
    consecutive_failures: consecutiveFailures,
    cooldown_remaining_seconds: cooldownRemaining,
    signals_detected: {
      has_failure: hasFailure || false,
      has_low_confidence: hasLowConfidence || false,
    },
  };
}

// Calculate run overview metrics
interface RunOverview {
  status: string;
  phase: string;
  overall_confidence: number;
  revision_count: number;
  total_failures: number;
}

function calculateRunOverview(
  runState: RunState,
  events: RunEvent[],
  beliefState: BeliefState | null
): RunOverview {
  const revisionInfo = parseRevisionState(events);
  const evaluations = parseEvaluations(events);
  const failures = evaluations.filter(e => !e.success).length;
  
  // Determine phase
  let phase = "idle";
  const lastEvent = events[events.length - 1];
  if (lastEvent) {
    if (lastEvent.event_type === "plan.created" || lastEvent.event_type === "plan.revised") {
      phase = "planning";
    } else if (lastEvent.event_type === "step.evaluated") {
      phase = "executing";
    } else if (lastEvent.event_type === "permission.requested") {
      phase = "waiting_permission";
    } else if (lastEvent.event_type === "input.requested") {
      phase = "waiting_input";
    }
  }
  
  return {
    status: runState.status,
    phase,
    overall_confidence: beliefState?.plan_confidence || 0.5,
    revision_count: revisionInfo.revision_count,
    total_failures: failures,
  };
}

export const CognitiveDebugConsole: React.FC<CognitiveDebugConsoleProps> = ({
  runState,
  events,
  beliefState,
}) => {
  const executionTimeline = useMemo(() => parseExecutionTimeline(events), [events]);
  const evaluations = useMemo(() => parseEvaluations(events), [events]);
  const failureAnalysis = useMemo(() => parseFailureAnalysis(events), [events]);
  const revisionInfo = useMemo(
    () => parseRevisionState(events, beliefState),
    [events, beliefState]
  );
  const runOverview = useMemo(
    () => calculateRunOverview(runState, events, beliefState),
    [runState, events, beliefState]
  );
  
  // Create evaluation map for quick lookup
  const evaluationMap = useMemo(() => {
    const map = new Map<string, Evaluation>();
    for (const eval_ of evaluations) {
      map.set(eval_.step_id, eval_);
    }
    return map;
  }, [evaluations]);
  
  return (
    <div className="w-full h-full flex flex-col gap-4 p-4 overflow-auto">
      {/* 1. Run Overview (Pi Agent) */}
      <Card className="p-4">
        <div className="flex items-center gap-2 mb-3">
          <h3 className="text-lg font-semibold">Run Overview</h3>
          <span className="text-xs font-medium px-2 py-0.5 rounded-full bg-primary/20 text-primary">
            Pi Agent
          </span>
        </div>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
          <div>
            <div className="text-muted-foreground">Status</div>
            <div className="font-medium">{runOverview.status}</div>
          </div>
          <div>
            <div className="text-muted-foreground">Phase</div>
            <div className="font-medium">{runOverview.phase}</div>
          </div>
          <div>
            <div className="text-muted-foreground">Confidence</div>
            <div className={`font-mono ${
              runOverview.overall_confidence > 0.7 ? 'text-green-600 dark:text-green-400' :
              runOverview.overall_confidence > 0.4 ? 'text-yellow-600 dark:text-yellow-400' :
              'text-red-600 dark:text-red-400'
            }`}>
              {(runOverview.overall_confidence * 100).toFixed(0)}%
            </div>
          </div>
          <div>
            <div className="text-muted-foreground">Revisions</div>
            <div className="font-medium">{runOverview.revision_count}</div>
          </div>
          <div>
            <div className="text-muted-foreground">Failures</div>
            <div className="font-medium">{runOverview.total_failures}</div>
          </div>
        </div>
      </Card>
      
      {/* 2. Current Plan */}
      {runState.plan && (
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Current Plan</h3>
          <div className="space-y-2">
            <div className="text-sm">
              <span className="font-medium">Summary: </span>
              <span className="text-muted-foreground">{runState.plan.summary}</span>
            </div>
            <div className="text-xs text-muted-foreground">
              Created: {new Date(runState.plan.created_at).toLocaleString()}
              {runState.plan.revised_at && (
                <> • Revised: {new Date(runState.plan.revised_at).toLocaleString()}</>
              )}
            </div>
            <div className="mt-2 space-y-1">
              {runState.plan.steps.map((step, idx) => (
                <div key={idx} className="text-xs flex items-center gap-2">
                  <span className="font-mono text-muted-foreground">
                    {String(idx + 1).padStart(2, "0")}.
                  </span>
                  <span>{step}</span>
                </div>
              ))}
            </div>
          </div>
        </Card>
      )}
      
      {/* 3. Execution Timeline + Evaluation (Side-by-Side) */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Execution Timeline */}
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Execution Timeline</h3>
          <div className="space-y-3 min-h-80 max-h-[60vh] overflow-auto">
            {executionTimeline.length === 0 ? (
              <div className="text-xs text-muted-foreground">No executions yet</div>
            ) : (
              executionTimeline.map((step, idx) => (
                <div key={step.step_id} className="border rounded p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="font-medium text-sm">
                      Step {idx + 1}: {step.capability}
                    </div>
                    <Badge variant={step.attempts.some(a => a.success) ? "default" : "destructive"}>
                      {step.attempts.some(a => a.success) ? "Success" : "Failed"}
                    </Badge>
                  </div>
                  {step.intent && (
                    <div className="text-xs text-muted-foreground mb-2">
                      Intent: {step.intent}
                    </div>
                  )}
                  <div className="space-y-1 ml-4 border-l-2 pl-2">
                    {step.attempts.map((attempt, attemptIdx) => (
                      <div key={attemptIdx} className="text-xs">
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-muted-foreground">
                            Attempt {attempt.attempt_number}
                            {attempt.retry_count > 0 && ` (retry ${attempt.retry_count})`}
                          </span>
                          <span className={attempt.success ? "text-green-600" : "text-red-600"}>
                            {attempt.success ? "✅" : "❌"}
                          </span>
                        </div>
                        {!attempt.success && attempt.failure_kind && (
                          <div className="text-muted-foreground ml-4">
                            {attempt.failure_kind}: {attempt.reason}
                          </div>
                        )}
                        <div className="text-muted-foreground ml-4 text-[10px]">
                          {new Date(attempt.timestamp).toLocaleTimeString()}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))
            )}
          </div>
        </Card>
        
        {/* Evaluation Panel */}
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Evaluation Panel</h3>
          <div className="space-y-3 min-h-80 max-h-[60vh] overflow-auto">
            {evaluations.length === 0 ? (
              <div className="text-xs text-muted-foreground">No evaluations yet</div>
            ) : (
              evaluations.map((eval_, idx) => (
                <div key={idx} className="border rounded p-3">
                  <div className="flex items-center justify-between mb-2">
                    <div className="text-xs font-mono text-muted-foreground">
                      {eval_.step_id}
                    </div>
                    <Badge variant={eval_.success ? "default" : "destructive"}>
                      {eval_.success ? "Match" : "Mismatch"}
                    </Badge>
                  </div>
                  <div className="space-y-1 text-xs">
                    <div>
                      <span className="font-medium">Expected: </span>
                      <span className="text-muted-foreground">{eval_.expected_outcome}</span>
                    </div>
                    <div>
                      <span className="font-medium">Actual: </span>
                      <span className="text-muted-foreground">{eval_.actual_outcome}</span>
                    </div>
                    <div>
                      <span className="font-medium">Confidence: </span>
                      <span className="font-mono">{(eval_.confidence * 100).toFixed(0)}%</span>
                    </div>
                    <div>
                      <span className="font-medium">Reason: </span>
                      <span className="text-muted-foreground">{eval_.reason}</span>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </Card>
      </div>
      
      {/* 4. Failure Analysis & Recovery */}
      {failureAnalysis.length > 0 && (
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Failure Analysis & Recovery</h3>
          <div className="space-y-2">
            {failureAnalysis.map((failure, idx) => (
              <div key={idx} className="border rounded p-3">
                <div className="flex items-center gap-2 mb-2">
                  <Badge variant="destructive">{failure.failure_kind}</Badge>
                  <span className="text-xs font-mono text-muted-foreground">
                    {failure.step_id}
                  </span>
                </div>
                <div className="text-sm text-muted-foreground">{failure.reason}</div>
                <div className="text-xs text-muted-foreground mt-1">
                  {new Date(failure.timestamp).toLocaleString()}
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}
      
      {/* 5. Belief State */}
      {beliefState && (
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Belief State</h3>
          <div className="space-y-3">
            {beliefState.current_hypothesis && (
              <div>
                <div className="text-sm font-medium mb-1">Current Hypothesis</div>
                <div className="text-sm text-muted-foreground">{beliefState.current_hypothesis}</div>
              </div>
            )}
            <div>
              <div className="text-sm font-medium mb-1">Plan Confidence</div>
              <div className={`text-lg font-mono ${
                beliefState.plan_confidence > 0.7 ? 'text-green-600 dark:text-green-400' :
                beliefState.plan_confidence > 0.4 ? 'text-yellow-600 dark:text-yellow-400' :
                'text-red-600 dark:text-red-400'
              }`}>
                {(beliefState.plan_confidence * 100).toFixed(1)}%
              </div>
            </div>
            {beliefState.known_failures.length > 0 && (
              <div>
                <div className="text-sm font-medium mb-1">Known Failures</div>
                <ul className="list-disc list-inside text-xs text-muted-foreground space-y-1">
                  {beliefState.known_failures.map((failure, idx) => (
                    <li key={idx}>{failure}</li>
                  ))}
                </ul>
              </div>
            )}
            {beliefState.recent_lessons.length > 0 && (
              <div>
                <div className="text-sm font-medium mb-1">Recent Lessons</div>
                <ul className="list-disc list-inside text-xs text-muted-foreground space-y-1">
                  {beliefState.recent_lessons.map((lesson, idx) => (
                    <li key={idx}>{lesson}</li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </Card>
      )}
      
      {/* 6. Revision State Inspector */}
      <Card className="p-4">
        <h3 className="text-lg font-semibold mb-3">Revision State Inspector</h3>
        <div className="space-y-3">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <div className="text-muted-foreground">Revision Count</div>
              <div className="font-medium">{revisionInfo.revision_count}</div>
            </div>
            <div>
              <div className="text-muted-foreground">Last Revision</div>
              <div className="font-medium">
                {revisionInfo.last_revision_time
                  ? new Date(revisionInfo.last_revision_time).toLocaleTimeString()
                  : "Never"}
              </div>
            </div>
            <div>
              <div className="text-muted-foreground">Consecutive Failures</div>
              <div className="font-medium">{revisionInfo.consecutive_failures}</div>
            </div>
            <div>
              <div className="text-muted-foreground">Cooldown Remaining</div>
              <div className="font-medium">
                {revisionInfo.cooldown_remaining_seconds !== null
                  ? `${revisionInfo.cooldown_remaining_seconds}s`
                  : "N/A"}
              </div>
            </div>
          </div>
          
          {/* Revision Signals */}
          <div className="border rounded p-3 bg-muted/30">
            <div className="text-sm font-medium mb-2">Revision Signals</div>
            <div className="space-y-1 text-xs">
              <div className="flex items-center gap-2">
                <span>{revisionInfo.signals_detected.has_failure ? "✔" : "✖"}</span>
                <span>Failure detected</span>
              </div>
              <div className="flex items-center gap-2">
                <span>{revisionInfo.signals_detected.has_low_confidence ? "✔" : "✖"}</span>
                <span>Low confidence ({beliefState ? (beliefState.plan_confidence * 100).toFixed(0) + "%" : "N/A"})</span>
              </div>
              <div className="flex items-center gap-2">
                <span>{revisionInfo.cooldown_remaining_seconds === null || revisionInfo.cooldown_remaining_seconds === 0 ? "✔" : "✖"}</span>
                <span>
                  Cooldown expired
                  {revisionInfo.cooldown_remaining_seconds !== null && revisionInfo.cooldown_remaining_seconds > 0 && (
                    <span className="text-muted-foreground ml-1">
                      ({revisionInfo.cooldown_remaining_seconds}s remaining)
                    </span>
                  )}
                </span>
              </div>
              <div className="mt-2 pt-2 border-t">
                <div className="font-medium">
                  {revisionInfo.signals_detected.has_failure && 
                   revisionInfo.signals_detected.has_low_confidence &&
                   (revisionInfo.cooldown_remaining_seconds === null || revisionInfo.cooldown_remaining_seconds === 0)
                    ? "→ Revision would be allowed"
                    : "→ Revision blocked"}
                </div>
              </div>
            </div>
          </div>
          
          {revisionInfo.last_revision_reason && (
            <div className="text-xs text-muted-foreground">
              Last revision reason: {revisionInfo.last_revision_reason}
            </div>
          )}
        </div>
      </Card>
      
      {/* 7. Reflection & Learning */}
      {beliefState && beliefState.recent_lessons.length > 0 && (
        <Card className="p-4">
          <h3 className="text-lg font-semibold mb-3">Reflection & Learning</h3>
          <div className="space-y-2">
            {events
              .filter(e => e.event_type === "run.reflected")
              .slice(-1)
              .map((event, idx) => (
                <div key={idx} className="border rounded p-3">
                  <div className="text-sm font-medium mb-2">Last Reflection</div>
                  <div className="text-xs text-muted-foreground mb-2">
                    {new Date(event.created_at).toLocaleString()}
                  </div>
                  {event.payload.lessons_learned && (
                    <div className="space-y-1">
                      <div className="text-xs font-medium">Lessons Learned:</div>
                      <ul className="list-disc list-inside text-xs text-muted-foreground">
                        {(event.payload.lessons_learned as string[]).map((lesson, i) => (
                          <li key={i}>{lesson}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              ))}
            <div className="text-sm font-medium mt-3">Applied Lessons</div>
            <ul className="list-disc list-inside text-xs text-muted-foreground space-y-1">
              {beliefState.recent_lessons.map((lesson, idx) => (
                <li key={idx}>{lesson}</li>
              ))}
            </ul>
          </div>
        </Card>
      )}
      
      {/* 8. Event Trace (Collapsed by default) */}
      <Card className="p-4">
        <details>
          <summary className="text-lg font-semibold cursor-pointer">Event Trace ({events.length})</summary>
          <div className="mt-3 space-y-1 min-h-[28rem] max-h-[70vh] overflow-auto text-xs">
            {events.map((event) => (
              <div key={event.id} className="p-2 bg-muted/50 rounded">
                <div className="flex items-center gap-2">
                  <span className="font-mono">{event.event_type}</span>
                  <span className="text-muted-foreground">
                    {new Date(event.created_at).toLocaleTimeString()}
                  </span>
                </div>
                <details className="mt-1">
                  <summary className="text-muted-foreground cursor-pointer">Payload</summary>
                  <pre className="mt-1 text-[10px] overflow-auto">
                    {JSON.stringify(event.payload, null, 2)}
                  </pre>
                </details>
              </div>
            ))}
          </div>
        </details>
      </Card>
    </div>
  );
};
