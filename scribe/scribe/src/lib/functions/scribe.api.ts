import { invoke } from "@tauri-apps/api/core";
import { safeLocalStorage } from "../storage";
import { STORAGE_KEYS } from "@/config";

/**
 * Check if Scribe API should be used.
 * @param selectedProviderFromCaller - The actual selected provider from the caller (React context).
 *   Use this as source of truth to avoid localStorage sync delays.
 */
export async function shouldUseScribeAPI(
  selectedProviderFromCaller?: { provider: string }
): Promise<boolean> {
  try {
    const ScribeApiEnabled =
      safeLocalStorage.getItem(STORAGE_KEYS.Scribe_API_ENABLED) === "true";
    if (!ScribeApiEnabled) return false;

    const hasLicense = await invoke<boolean>("check_license_status");
    if (!hasLicense) return false;

    // Use caller's selection as source of truth (avoids localStorage sync delay)
    const providerId = selectedProviderFromCaller?.provider;
    if (providerId) {
      if (providerId === "ollama" || providerId === "exora") {
        // Exora/Ollama: use Scribe API only when user has explicitly chosen a free model
        const storage = await invoke<{ selected_Scribe_model?: string }>("secure_storage_get");
        const hasSelectedFreeModel = !!(storage?.selected_Scribe_model?.trim());
        return hasSelectedFreeModel;
      }
      // GPT 4o Mini: use Scribe API when enabled (no user API key needed)
      if (providerId === "gpt-4o-mini") return true;
      // Non-Exora provider (OpenAI, Claude, etc.): always use direct path.
      return false;
    }

    // Fallback: no provider from caller, check localStorage
    const selectedProviderJson = safeLocalStorage.getItem(STORAGE_KEYS.SELECTED_AI_PROVIDER);
    if (selectedProviderJson) {
      try {
        const p = JSON.parse(selectedProviderJson);
        if (p?.provider === "ollama" || p?.provider === "exora") {
          const storage = await invoke<{ selected_Scribe_model?: string }>("secure_storage_get");
          return !!(storage?.selected_Scribe_model?.trim());
        }
        if (p?.provider === "gpt-4o-mini") return true;
        return false;
      } catch {}
    }
    return false;
  } catch (error) {
    console.warn("Failed to check Scribe API availability:", error);
    return false;
  }
}
