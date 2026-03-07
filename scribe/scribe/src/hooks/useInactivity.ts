import { useEffect, useRef, useCallback } from "react";

const DEFAULT_TIMEOUT_MS = 25000;

/**
 * Tracks user activity and invokes onInactive when no activity for timeoutMs.
 * Activity: mousemove, mousedown, keydown, focus.
 * Resets timer on any activity.
 */
export const useInactivity = (
  onInactive: () => void,
  options?: {
    timeoutMs?: number;
    enabled?: boolean;
  }
) => {
  const timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const enabled = options?.enabled ?? true;
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onInactiveRef = useRef(onInactive);
  onInactiveRef.current = onInactive;

  const resetTimer = useCallback(() => {
    if (!enabled) return;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    timeoutRef.current = setTimeout(() => {
      timeoutRef.current = null;
      onInactiveRef.current();
    }, timeoutMs);
  }, [timeoutMs, enabled]);

  useEffect(() => {
    if (!enabled) {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      return;
    }

    resetTimer();

    const handleActivity = () => resetTimer();

    window.addEventListener("mousemove", handleActivity);
    window.addEventListener("mousedown", handleActivity);
    window.addEventListener("keydown", handleActivity);
    window.addEventListener("focus", handleActivity);
    window.addEventListener("user-activity", handleActivity);

    return () => {
      window.removeEventListener("mousemove", handleActivity);
      window.removeEventListener("mousedown", handleActivity);
      window.removeEventListener("keydown", handleActivity);
      window.removeEventListener("focus", handleActivity);
      window.removeEventListener("user-activity", handleActivity);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [enabled, resetTimer]);

  return { resetTimer };
};
