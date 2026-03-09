/**
 * Central constants for the Scribe app.
 * Production URLs come from VITE_* env vars at build time.
 */

/** Gateway WebSocket URL - use VITE_GHOST_GATEWAY_WS_URL for production (e.g. wss://api.ghost.exora.solutions/gateway) */
export const GHOST_GATEWAY_WS_URL =
  (typeof import.meta !== "undefined" && (import.meta as { env?: Record<string, string> }).env?.VITE_GHOST_GATEWAY_WS_URL) ||
  "ws://127.0.0.1:8083/gateway";

/** Scribe API base URL - use VITE_APP_ENDPOINT for production (e.g. https://api.ghost.exora.solutions) */
export const APP_ENDPOINT =
  (typeof import.meta !== "undefined" && (import.meta as { env?: Record<string, string> }).env?.VITE_APP_ENDPOINT) ||
  "http://127.0.0.1:8083";

export const STORAGE_KEYS = {
  THEME: "scribe_theme",
  TRANSPARENCY: "scribe_transparency",
  SYSTEM_PROMPT: "scribe_system_prompt",
  SELECTED_SYSTEM_PROMPT_ID: "scribe_selected_system_prompt_id",
  SELECTED_AI_PROVIDER: "scribe_selected_ai_provider",
  CUSTOM_AI_PROVIDERS: "scribe_custom_ai_providers",
  CUSTOM_SPEECH_PROVIDERS: "scribe_custom_speech_providers",
  SHORTCUTS: "scribe_shortcuts",
  SCREENSHOT_CONFIG: "scribe_screenshot_config",
  CUSTOMIZABLE: "scribe_customizable",
  VOICE_ACTIVATION: "scribe_voice_activation",
  Scribe_API_ENABLED: "scribe_api_enabled",
  SYSTEM_AUDIO_CONTEXT: "scribe_system_audio_context",
  SYSTEM_AUDIO_QUICK_ACTIONS: "scribe_system_audio_quick_actions",
  SELECTED_AUDIO_INPUT_DEVICE: "scribe_selected_audio_input_device",
  SELECTED_AUDIO_OUTPUT_DEVICE: "scribe_selected_audio_output_device",
} as const;

export const MAX_FILES = 6;

export const DEFAULT_SYSTEM_PROMPT = `You are Scribe, a helpful AI assistant. Be concise and direct.`;
