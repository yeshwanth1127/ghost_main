import { useEffect, useRef, useCallback } from "react";

const DEFAULT_PHRASES = ["ghost", "hey ghost"];

interface SpeechRecognitionResultItem {
  isFinal: boolean;
  0: { transcript: string };
  length: number;
}

interface SpeechRecognitionInstance {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start: () => void;
  stop: () => void;
  abort: () => void;
  onresult: ((event: { resultIndex: number; results: SpeechRecognitionResultItem[] }) => void) | null;
  onend: (() => void) | null;
  onerror: ((event: { error: string }) => void) | null;
}

const getSpeechRecognition = (): (new () => SpeechRecognitionInstance) | undefined => {
  if (typeof window === "undefined") return undefined;
  return (
    (window as any).SpeechRecognition ?? (window as any).webkitSpeechRecognition
  );
};

const isSupported = (): boolean => !!getSpeechRecognition();

/**
 * Listens for wake phrases via Web Speech API when active.
 * Calls onPhraseDetected when a matching phrase is heard.
 */
export const useVoiceActivation = (
  onPhraseDetected: () => void,
  options?: {
    enabled?: boolean;
    phrases?: string[];
    customPhrase?: string;
  }
) => {
  const enabled = options?.enabled ?? false;
  const customPhrase = options?.customPhrase?.trim().toLowerCase();
  const defaultPhrases = options?.phrases ?? DEFAULT_PHRASES;

  const phrases = customPhrase
    ? [...defaultPhrases, customPhrase]
    : defaultPhrases;

  const recognitionRef = useRef<SpeechRecognitionInstance | null>(null);
  const onPhraseDetectedRef = useRef(onPhraseDetected);
  const enabledRef = useRef(enabled);
  onPhraseDetectedRef.current = onPhraseDetected;
  enabledRef.current = enabled;

  const checkTranscript = useCallback(
    (transcript: string) => {
      const normalized = transcript.trim().toLowerCase();
      const match = phrases.some((p) => normalized.includes(p));
      if (match) {
        onPhraseDetectedRef.current();
      }
    },
    [phrases]
  );

  useEffect(() => {
    if (!enabled || !isSupported()) return;

    const SpeechRecognitionClass = getSpeechRecognition();
    if (!SpeechRecognitionClass) return;

    const recognition = new SpeechRecognitionClass() as SpeechRecognitionInstance;
    recognitionRef.current = recognition;

    recognition.continuous = true;
    recognition.interimResults = false;
    recognition.lang = "en-US";

    recognition.onresult = (event: { resultIndex: number; results: SpeechRecognitionResultItem[] }) => {
      for (let i = event.resultIndex; i < event.results.length; i++) {
        const result = event.results[i];
        const transcript = result[0]?.transcript ?? "";
        if (result.isFinal && transcript) {
          checkTranscript(transcript);
        }
      }
    };

    recognition.onend = () => {
      if (enabledRef.current && recognitionRef.current) {
        try {
          recognition.start();
        } catch {
          // Already started or error - ignore
        }
      }
    };

    recognition.onerror = (event: { error: string }) => {
      if (event.error === "not-allowed" || event.error === "service-not-allowed") {
        console.warn("Voice activation: microphone access denied");
      }
    };

    try {
      recognition.start();
    } catch (e) {
      console.warn("Voice activation: failed to start", e);
    }

    return () => {
      recognitionRef.current = null;
      try {
        recognition.stop();
        recognition.abort();
      } catch {
        // Ignore
      }
    };
  }, [enabled, checkTranscript]);

  return { isSupported: isSupported() };
};
