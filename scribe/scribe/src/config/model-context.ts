/**
 * Model-aware context budget for rolling window trimming.
 * Replaces hardcoded MAX_CONTEXT_TOKENS with dynamic limits per model.
 */

export const MODEL_CONTEXT_LIMITS: Record<string, number> = {
  "gpt-4o-mini": 128000,
  "gpt-4o": 128000,
  "gpt-4-turbo": 128000,
  "gpt-4": 128000,
  "gpt-3.5-turbo": 16385,
  "claude-3-5-sonnet": 200000,
  "claude-3-5-sonnet-20241022": 200000,
  "claude-3-opus": 200000,
  "claude-3-sonnet": 200000,
  "claude-3-haiku": 200000,
  "deepseek-v3": 128000,
  "deepseek-chat": 128000,
  "llama3": 128000,
  "llama3.1": 128000,
  "llama3.2": 128000,
  "mistral-large": 128000,
  "mistral-small": 32000,
  "gemini-1.5-pro": 1000000,
  "gemini-1.5-flash": 1000000,
  "gemini-pro": 32000,
};

export const CONTEXT_BUDGET_CAP = 8000;
export const DEFAULT_CONTEXT_BUDGET = 4000;

/**
 * Get context budget (max tokens for history) for a model.
 * Uses 70% of model limit, capped at 8000 to control costs.
 */
export function getContextBudget(modelId: string | undefined): number {
  const limit = modelId ? MODEL_CONTEXT_LIMITS[modelId] : undefined;
  const raw = limit ? limit * 0.7 : DEFAULT_CONTEXT_BUDGET;
  return Math.min(raw, CONTEXT_BUDGET_CAP);
}
