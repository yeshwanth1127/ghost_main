import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

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
}

export const AgentView = () => {
  const [runs, setRuns] = useState<string[]>([]);
  const [selectedRun, setSelectedRun] = useState<string | null>(null);
  const [runState, setRunState] = useState<RunState | null>(null);
  const [events, setEvents] = useState<RunEvent[]>([]);
  const [goal, setGoal] = useState("");

  // Subscribe to run events
  useEffect(() => {
    const setupEventListener = async () => {
      const unlisten = await listen<RunEvent>("run_event", (event) => {
        const runEvent = event.payload;
        if (runEvent.run_id === selectedRun) {
          // Reload state when event arrives
          if (selectedRun) {
            loadRunState(selectedRun);
          }
        }
      });

      return () => {
        unlisten();
      };
    };

    setupEventListener();
  }, [selectedRun]);

  const loadRuns = async () => {
    // Note: Run list functionality will be added when needed
    // Currently, runs are created and selected individually
  };

  const loadRunState = async (runId: string) => {
    try {
      const state = await invoke<RunState>("get_run_state", { runId });
      setRunState(state);
      
      const runEvents = await invoke<RunEvent[]>("get_run_events", { runId });
      setEvents(runEvents);
    } catch (error) {
      console.error("Failed to load run state:", error);
    }
  };

  const createRun = async () => {
    if (!goal.trim()) return;
    
    try {
      const runId = await invoke<string>("create_run", { goal });
      setRuns([...runs, runId]);
      setSelectedRun(runId);
      setGoal("");
      
      // Start the run
      await invoke("start_run", { runId });
      
      // Load initial state
      await loadRunState(runId);
    } catch (error) {
      console.error("Failed to create run:", error);
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

  return (
    <div className="w-full h-full flex flex-col gap-4 p-4">
      <div className="flex flex-row gap-4">
        <Card className="flex-1 p-4">
          <h2 className="text-lg font-semibold mb-4">Create New Run</h2>
          <div className="flex gap-2">
            <input
              type="text"
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
              placeholder="Enter goal for the agent..."
              className="flex-1 px-3 py-2 border rounded"
              onKeyPress={(e) => {
                if (e.key === "Enter") {
                  createRun();
                }
              }}
            />
            <Button onClick={createRun} disabled={!goal.trim()}>
              Start Run
            </Button>
          </div>
        </Card>
      </div>

      {selectedRun && runState && (
        <Card className="flex-1 p-4 overflow-auto">
          <h3 className="text-md font-semibold mb-2">Run: {runState.goal}</h3>
          <div className="text-sm text-muted-foreground mb-4">
            Status: {runState.status}
          </div>

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
                          {step.tool_name || "Unknown"}
                        </div>
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
            <div className="space-y-1 max-h-64 overflow-auto">
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
