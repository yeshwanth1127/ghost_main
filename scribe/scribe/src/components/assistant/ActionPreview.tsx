import { useState, useEffect } from "react";
import type { PreviewResult, ActionPlan, Action } from "@/types/assistant";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { AlertTriangle, CheckCircle, XCircle } from "lucide-react";
import { PathPromptDialog } from "./PathPromptDialog";

interface ActionPreviewProps {
  preview: PreviewResult;
  onApprove: (plan: ActionPlan) => void;
  onReject: () => void;
  onEdit?: (plan: ActionPlan) => void;
  requiresExplicitConfirmation?: boolean;
}

export function ActionPreview({
  preview,
  onApprove,
  onReject,
  onEdit,
  requiresExplicitConfirmation = false,
}: ActionPreviewProps) {
  const [confirmationText, setConfirmationText] = useState("");
  const [selectedActions, setSelectedActions] = useState<Set<string>>(
    new Set(preview.plan.actions.map((a) => a.id))
  );
  const [pathDialogOpen, setPathDialogOpen] = useState(false);
  const [currentActionIndex, setCurrentActionIndex] = useState(0);
  const [updatedPlan, setUpdatedPlan] = useState<ActionPlan>(preview.plan);
  const [pathsProvided, setPathsProvided] = useState<Set<string>>(new Set());

  // Open path dialog if there are missing paths
  useEffect(() => {
    if (preview.missing_paths.length > 0 && currentActionIndex < preview.missing_paths.length) {
      setPathDialogOpen(true);
    }
  }, [preview.missing_paths, currentActionIndex]);

  const handlePathProvided = (path: string) => {
    const actionId = preview.missing_paths[currentActionIndex];
    const action = updatedPlan.actions.find((a) => a.id === actionId);
    
    if (!action) return;

        // Update the action with the provided path
        const updatedActions = updatedPlan.actions.map((a) => {
          if (a.id === actionId) {
            const newArgs = { ...a.args };
            // Replace placeholder paths
            if (a.args.path === "__PROMPT_PATH__" || a.args.path === "[Path needed]") {
              newArgs.path = path;
            }
            if (a.args.destination_path === "__PROMPT_PATH__" || a.args.destination_path === "[Destination path needed]") {
              newArgs.destination_path = path;
            }
            if (a.args.source_path === "__PROMPT_PATH__") {
              newArgs.source_path = path;
            }
            return { ...a, args: newArgs };
          }
          return a;
        });

    const newPlan = { ...updatedPlan, actions: updatedActions };
    setUpdatedPlan(newPlan);
    setPathsProvided(new Set([...pathsProvided, actionId]));

    // Move to next missing path or close dialog
    if (currentActionIndex < preview.missing_paths.length - 1) {
      setCurrentActionIndex(currentActionIndex + 1);
    } else {
      setPathDialogOpen(false);
    }
  };

  const getCurrentAction = (): Action | null => {
    if (preview.missing_paths.length === 0) return null;
    const actionId = preview.missing_paths[currentActionIndex];
    return updatedPlan.actions.find((a) => a.id === actionId) || null;
  };

  const currentAction = getCurrentAction();

  const riskColor =
    preview.risk_score < 0.3
      ? "text-green-600"
      : preview.risk_score < 0.7
      ? "text-yellow-600"
      : "text-red-600";

  const handleToggleAction = (actionId: string) => {
    const newSelected = new Set(selectedActions);
    if (newSelected.has(actionId)) {
      newSelected.delete(actionId);
    } else {
      newSelected.add(actionId);
    }
    setSelectedActions(newSelected);
  };

  const handleApprove = () => {
    // Check if all paths are provided
    if (preview.missing_paths.length > 0 && pathsProvided.size < preview.missing_paths.length) {
      // Open dialog for first missing path
      setCurrentActionIndex(0);
      setPathDialogOpen(true);
      return;
    }

    if (requiresExplicitConfirmation) {
      // For high-risk actions, require typing the summary or a keyword
      const requiredText = preview.plan.summary.toLowerCase();
      if (confirmationText.toLowerCase() !== requiredText) {
        return;
      }
    }

    // Filter actions to only approved ones
    const approvedPlan: ActionPlan = {
      ...updatedPlan,
      actions: updatedPlan.actions.filter((a) =>
        selectedActions.has(a.id)
      ),
    };

    onApprove(approvedPlan);
  };

  const allPathsProvided = 
    preview.missing_paths.length === 0 || pathsProvided.size === preview.missing_paths.length;

  const canApprove =
    selectedActions.size > 0 &&
    allPathsProvided &&
    (!requiresExplicitConfirmation ||
      confirmationText.toLowerCase() === preview.plan.summary.toLowerCase());

  return (
    <Card className="p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Action Preview</h3>
        <div className="flex items-center gap-2">
          <span className={`text-sm font-medium ${riskColor}`}>
            Risk: {(preview.risk_score * 100).toFixed(0)}%
          </span>
          {preview.risk_score > 0.7 && (
            <AlertTriangle className="w-4 h-4 text-red-600" />
          )}
        </div>
      </div>

      <div>
        <p className="text-sm text-muted-foreground">{preview.plan.summary}</p>
      </div>

      {preview.missing_paths.length > 0 && (
        <div className="bg-blue-50 border border-blue-200 rounded p-3">
          <h4 className="text-sm font-medium text-blue-800 mb-2">
            Path Information Needed:
          </h4>
          <p className="text-sm text-blue-700">
            {preview.missing_paths.length} path{preview.missing_paths.length > 1 ? "s" : ""} need{preview.missing_paths.length === 1 ? "s" : ""} to be provided
            {pathsProvided.size > 0 && ` (${pathsProvided.size}/${preview.missing_paths.length} provided)`}
          </p>
        </div>
      )}

      {preview.warnings.length > 0 && (
        <div className="bg-yellow-50 border border-yellow-200 rounded p-3">
          <h4 className="text-sm font-medium text-yellow-800 mb-2">
            Warnings:
          </h4>
          <ul className="list-disc list-inside text-sm text-yellow-700 space-y-1">
            {preview.warnings.map((warning, idx) => (
              <li key={idx}>{warning}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="space-y-2">
        <h4 className="text-sm font-medium">Affected Items:</h4>
        {preview.affected_items.map((item, idx) => {
          const action = preview.plan.actions[idx];
          const isSelected = action && selectedActions.has(action.id);

          return (
            <div
              key={idx}
              className={`flex items-center gap-2 p-2 rounded border ${
                isSelected ? "bg-blue-50 border-blue-200" : "bg-gray-50"
              }`}
            >
              <input
                type="checkbox"
                checked={isSelected}
                onChange={() => action && handleToggleAction(action.id)}
                className="rounded"
              />
              <div className="flex-1">
                <p className="text-sm font-mono">{item.path}</p>
                <p className="text-xs text-muted-foreground">{item.operation}</p>
              </div>
            </div>
          );
        })}
      </div>

      {requiresExplicitConfirmation && (
        <div className="space-y-2">
          <p className="text-sm text-red-600 font-medium">
            High-risk action: Please type "{preview.plan.summary}" to confirm:
          </p>
          <input
            type="text"
            value={confirmationText}
            onChange={(e) => setConfirmationText(e.target.value)}
            className="w-full px-3 py-2 border rounded"
            placeholder={preview.plan.summary}
          />
        </div>
      )}

      <div className="flex gap-2 justify-end">
        <Button variant="outline" onClick={onReject}>
          <XCircle className="w-4 h-4 mr-2" />
          Reject
        </Button>
        {onEdit && (
          <Button variant="outline" onClick={() => onEdit(preview.plan)}>
            Edit
          </Button>
        )}
        <Button
          onClick={handleApprove}
          disabled={!canApprove}
          variant={preview.risk_score > 0.7 ? "destructive" : "default"}
        >
          <CheckCircle className="w-4 h-4 mr-2" />
          {preview.missing_paths.length > pathsProvided.size ? "Provide Paths" : "Approve"}
        </Button>
      </div>

      {currentAction && (
        <PathPromptDialog
          open={pathDialogOpen}
          onOpenChange={setPathDialogOpen}
          actionType={currentAction.type || "file operation"}
          onConfirm={handlePathProvided}
          isDirectory={currentAction.type?.includes("directory") || false}
          isDestination={currentAction.args?.destination_path === "__PROMPT_PATH__"}
        />
      )}
    </Card>
  );
}

