import { invoke } from "@tauri-apps/api/core";
import { safeLocalStorage } from "../storage";
import { STORAGE_KEYS } from "@/config";

// Helper function to check if Scribe API should be used
export async function shouldUseScribeAPI(): Promise<boolean> {
  try {
    // Check if Scribe API is enabled in localStorage
    const ScribeApiEnabled =
      safeLocalStorage.getItem(STORAGE_KEYS.Scribe_API_ENABLED) === "true";
    if (!ScribeApiEnabled) return false;

    // Check if license is available
    const hasLicense = await invoke<boolean>("check_license_status");
    return hasLicense;
  } catch (error) {
    console.warn("Failed to check Scribe API availability:", error);
    return false;
  }
}
