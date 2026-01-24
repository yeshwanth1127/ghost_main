import { useState } from "react";
import { useActionAssistant } from "@/hooks/useActionAssistant";
import { ActionPreview } from "./ActionPreview";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Loader2, Sparkles } from "lucide-react";
import type { ActionPlan, PreviewResult } from "@/types/assistant";

export function ActionMode() {
  const {
    planAction,
    previewAction,
    executeAction,
    isPlanning,
    isExecuting,
    error,
    clearError,
  } = useActionAssistant();

  const [input, setInput] = useState("");
  const [useLLM, setUseLLM] = useState(false);
  const [previewResult, setPreviewResult] = useState<PreviewResult | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    clearError();

    try {
      const plan = await planAction(input, useLLM);
      const preview = await previewAction(plan);
      setPreviewResult(preview);
    } catch (err) {
      console.error("Failed to plan action:", err);
    }
  };

  const handleApprove = async (plan: ActionPlan) => {
    try {
      await executeAction(plan);
      setInput("");
      setPreviewResult(null);
    } catch (err) {
      console.error("Failed to execute action:", err);
    }
  };

  const handleReject = () => {
    setPreviewResult(null);
    setInput("");
  };

  return (
    <div className="w-full space-y-4 p-4">
      <Card className="p-4">
        <form onSubmit={handleSubmit} className="space-y-3">
          <div className="flex gap-2">
            <Input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Enter action request (e.g., 'create file notes.txt with hello world')"
              disabled={isPlanning || isExecuting}
              className="flex-1"
            />
            <Button
              type="submit"
              disabled={isPlanning || isExecuting || !input.trim()}
            >
              {isPlanning ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Planning...
                </>
              ) : (
                <>
                  <Sparkles className="w-4 h-4 mr-2" />
                  Plan
                </>
              )}
            </Button>
          </div>

          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="use-llm"
              checked={useLLM}
              onChange={(e) => setUseLLM(e.target.checked)}
              className="rounded"
            />
            <label htmlFor="use-llm" className="text-sm text-muted-foreground">
              Use AI for complex planning
            </label>
          </div>
        </form>

        {error && (
          <div className="mt-3 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-700">
            {error}
          </div>
        )}
      </Card>

      {previewResult && (
        <ActionPreview
          preview={previewResult}
          onApprove={handleApprove}
          onReject={handleReject}
          requiresExplicitConfirmation={previewResult.requires_explicit_confirmation}
        />
      )}

      {isExecuting && (
        <Card className="p-4">
          <div className="flex items-center gap-2">
            <Loader2 className="w-4 h-4 animate-spin" />
            <span>Executing action...</span>
          </div>
        </Card>
      )}
    </div>
  );
}

