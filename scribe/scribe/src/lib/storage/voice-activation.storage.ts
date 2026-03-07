import { STORAGE_KEYS } from "@/config";

export interface VoiceActivationState {
  enabled: boolean;
  customPhrase: string;
  autoCollapseEnabled: boolean;
  logoPosition: { x: number; y: number } | null;
}

export const DEFAULT_VOICE_ACTIVATION_STATE: VoiceActivationState = {
  enabled: true,
  customPhrase: "",
  autoCollapseEnabled: true,
  logoPosition: null,
};

export const getVoiceActivationState = (): VoiceActivationState => {
  try {
    const stored = localStorage.getItem(STORAGE_KEYS.VOICE_ACTIVATION);
    if (!stored) {
      return DEFAULT_VOICE_ACTIVATION_STATE;
    }

    const parsed = JSON.parse(stored);
    return {
      enabled: parsed.enabled ?? DEFAULT_VOICE_ACTIVATION_STATE.enabled,
      customPhrase: parsed.customPhrase ?? DEFAULT_VOICE_ACTIVATION_STATE.customPhrase,
      autoCollapseEnabled:
        parsed.autoCollapseEnabled ?? DEFAULT_VOICE_ACTIVATION_STATE.autoCollapseEnabled,
      logoPosition: parsed.logoPosition ?? DEFAULT_VOICE_ACTIVATION_STATE.logoPosition,
    };
  } catch (error) {
    console.error("Failed to get voice activation state:", error);
    return DEFAULT_VOICE_ACTIVATION_STATE;
  }
};

export const setVoiceActivationState = (state: VoiceActivationState): void => {
  try {
    localStorage.setItem(STORAGE_KEYS.VOICE_ACTIVATION, JSON.stringify(state));
  } catch (error) {
    console.error("Failed to save voice activation state:", error);
  }
};

export const updateVoiceActivationEnabled = (enabled: boolean): VoiceActivationState => {
  const current = getVoiceActivationState();
  const newState = { ...current, enabled };
  setVoiceActivationState(newState);
  return newState;
};

export const updateVoiceActivationCustomPhrase = (customPhrase: string): VoiceActivationState => {
  const current = getVoiceActivationState();
  const newState = { ...current, customPhrase };
  setVoiceActivationState(newState);
  return newState;
};

export const updateVoiceActivationAutoCollapse = (autoCollapseEnabled: boolean): VoiceActivationState => {
  const current = getVoiceActivationState();
  const newState = { ...current, autoCollapseEnabled };
  setVoiceActivationState(newState);
  return newState;
};

export const updateLogoPosition = (x: number, y: number): VoiceActivationState => {
  const current = getVoiceActivationState();
  const newState = { ...current, logoPosition: { x, y } };
  setVoiceActivationState(newState);
  return newState;
};
