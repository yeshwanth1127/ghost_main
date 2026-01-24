import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ActionPlan,
  PreviewResult,
  ActionResult,
  AuditEntry,
} from "@/types/assistant";
import { planWithLLM } from "@/lib/functions/action-planner.function";
import type { Message } from "@/types";
import { saveAuditLogCache } from "@/lib/database/action-history.action";

interface ActionAssistantState {
  currentPlan: ActionPlan | null;
  preview: PreviewResult | null;
  isPlanning: boolean;
  isExecuting: boolean;
  lastResult: ActionResult | null;
  error: string | null;
}

export function useActionAssistant() {
  const [state, setState] = useState<ActionAssistantState>({
    currentPlan: null,
    preview: null,
    isPlanning: false,
    isExecuting: false,
    lastResult: null,
    error: null,
  });

  // const abortControllerRef = useRef<AbortController | null>(null);

  /// Parse intent using deterministic parser
  const parseIntent = useCallback(async (input: string): Promise<ActionPlan> => {
    setState((prev) => ({ ...prev, isPlanning: true, error: null }));

    try {
      const plan = await invoke<ActionPlan>("parse_intent", { userInput: input });
      setState((prev) => ({
        ...prev,
        currentPlan: plan,
        isPlanning: false,
      }));
      return plan;
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : String(error);
      setState((prev) => ({
        ...prev,
        error: errorMsg,
        isPlanning: false,
      }));
      throw new Error(errorMsg);
    }
  }, []);

  /// Plan action (deterministic or LLM)
  const planAction = useCallback(
    async (
      input: string,
      useLLM: boolean = false,
      history: Message[] = []
    ): Promise<ActionPlan> => {
      setState((prev) => ({ ...prev, isPlanning: true, error: null }));

      try {
        let plan: ActionPlan;

        if (useLLM) {
          // Use LLM planner
          plan = await planWithLLM(input, history);
          
          // Validate the plan via backend
          plan = await invoke<ActionPlan>("verify_action_plan", { plan });
        } else {
          // Use deterministic parser
          plan = await parseIntent(input);
        }

        setState((prev) => ({
          ...prev,
          currentPlan: plan,
          isPlanning: false,
        }));

        return plan;
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : String(error);
        setState((prev) => ({
          ...prev,
          error: errorMsg,
          isPlanning: false,
        }));
        throw error;
      }
    },
    [parseIntent]
  );

  /// Preview an action plan
  const previewAction = useCallback(
    async (plan: ActionPlan): Promise<PreviewResult> => {
      try {
        const preview = await invoke<PreviewResult>("preview_action_plan", {
          plan,
        });
        setState((prev) => ({
          ...prev,
          preview,
          currentPlan: plan,
        }));
        return preview;
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : String(error);
        setState((prev) => ({
          ...prev,
          error: errorMsg,
        }));
        throw new Error(errorMsg);
      }
    },
    []
  );

  /// Execute an action plan
  const executeAction = useCallback(
    async (plan: ActionPlan, confirmToken?: string): Promise<ActionResult> => {
      setState((prev) => ({ ...prev, isExecuting: true, error: null }));

      try {
        const result = await invoke<ActionResult>("execute_action_plan", {
          plan,
          confirmToken: confirmToken || null,
        });

        // Save audit log entry to database (with hash chain)
        try {
          const auditEntry: AuditEntry = {
            id: crypto.randomUUID(),
            entry_json: JSON.stringify({ action_plan: plan, result }),
            timestamp: Math.floor(Date.now() / 1000),
            prev_hash: "", // Will be calculated by saveAuditLogCache from last entry
            signature: "", // Will be calculated by saveAuditLogCache
            action_id: plan.id,
          };
          await saveAuditLogCache(auditEntry);
        } catch (auditError) {
          console.warn("Failed to save audit log:", auditError);
          // Don't fail the action if audit save fails
        }

        setState((prev) => ({
          ...prev,
          lastResult: result,
          isExecuting: false,
          currentPlan: null,
          preview: null,
        }));

        return result;
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : String(error);
        setState((prev) => ({
          ...prev,
          error: errorMsg,
          isExecuting: false,
        }));
        throw new Error(errorMsg);
      }
    },
    []
  );

  /// Undo an action
  const undoAction = useCallback(async (actionId: string): Promise<void> => {
    try {
      await invoke("undo_action", { actionId });
      setState((prev) => ({
        ...prev,
        lastResult: null,
      }));
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : String(error);
      setState((prev) => ({
        ...prev,
        error: errorMsg,
      }));
      throw new Error(errorMsg);
    }
  }, []);

  /// Get audit history
  const getAuditHistory = useCallback(
    async (limit: number = 50): Promise<AuditEntry[]> => {
      try {
        // Use frontend database function instead of IPC
        const { getAuditLogsFromDB } = await import("@/lib/database/action-history.action");
        return await getAuditLogsFromDB(limit);
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : String(error);
        throw new Error(errorMsg);
      }
    },
    []
  );

  /// Clear error
  const clearError = useCallback(() => {
    setState((prev) => ({ ...prev, error: null }));
  }, []);

  return {
    // State
    ...state,
    // Actions
    parseIntent,
    planAction,
    previewAction,
    executeAction,
    undoAction,
    getAuditHistory,
    clearError,
  };
}

