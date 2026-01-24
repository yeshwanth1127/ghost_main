import { useEffect, useState } from "react";
import { useActionAssistant } from "@/hooks/useActionAssistant";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Undo2, RefreshCw, Loader2 } from "lucide-react";
import type { AuditEntry } from "@/types/assistant";
// Using basic date formatting instead of date-fns
const formatDate = (timestamp: number) => {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString();
};

export function ActionHistory() {
  const { getAuditHistory, undoAction } = useActionAssistant();
  const [history, setHistory] = useState<AuditEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [undoingId, setUndoingId] = useState<string | null>(null);

  const loadHistory = async () => {
    setLoading(true);
    try {
      const entries = await getAuditHistory(50);
      setHistory(entries);
    } catch (error) {
      console.error("Failed to load audit history:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadHistory();
  }, []);

  const handleUndo = async (actionId: string) => {
    setUndoingId(actionId);
    try {
      await undoAction(actionId);
      await loadHistory(); // Refresh list
    } catch (error) {
      console.error("Failed to undo action:", error);
    } finally {
      setUndoingId(null);
    }
  };

  if (loading) {
    return (
      <Card className="p-4">
        <div className="flex items-center gap-2">
          <Loader2 className="w-4 h-4 animate-spin" />
          <span>Loading history...</span>
        </div>
      </Card>
    );
  }

  return (
    <div className="w-full space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Action History</h3>
        <Button variant="outline" size="sm" onClick={loadHistory}>
          <RefreshCw className="w-4 h-4 mr-2" />
          Refresh
        </Button>
      </div>

      {history.length === 0 ? (
        <Card className="p-4 text-center text-muted-foreground">
          No actions executed yet
        </Card>
      ) : (
        <div className="space-y-2">
          {history.map((entry) => {
            let actionData: any = {};
            try {
              actionData = JSON.parse(entry.entry_json);
            } catch {}

            const actionPlan = actionData.action_plan;
            const actionId = actionPlan?.schema?.id || entry.action_id || entry.id;

            return (
              <Card key={entry.id} className="p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="text-sm font-mono text-muted-foreground">
                        {entry.action_id || entry.id.slice(0, 8)}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {formatDate(entry.timestamp)}
                      </span>
                    </div>
                    {actionPlan && (
                      <div className="text-sm">
                        <p className="font-medium">{actionPlan.schema?.summary || "Action executed"}</p>
                        {actionPlan.schema?.actions && (
                          <p className="text-muted-foreground mt-1">
                            {actionPlan.schema.actions.length} action(s)
                          </p>
                        )}
                      </div>
                    )}
                    {actionData.result && actionData.result.success && (
                      <div className="mt-2 text-xs text-green-600">
                        ✓ Successfully executed
                      </div>
                    )}
                    {actionData.result && !actionData.result.success && (
                      <div className="mt-2 text-xs text-red-600">
                        ✗ Execution failed
                      </div>
                    )}
                  </div>
                  {actionData.result?.undo_available && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleUndo(actionId)}
                      disabled={undoingId === actionId}
                    >
                      {undoingId === actionId ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <>
                          <Undo2 className="w-4 h-4 mr-2" />
                          Undo
                        </>
                      )}
                    </Button>
                  )}
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

