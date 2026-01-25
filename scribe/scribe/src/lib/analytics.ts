// Lazy import PostHog to avoid errors when plugin isn't initialized
let PostHog: any = null;
let posthogLoaded = false;

const loadPostHog = async () => {
  if (posthogLoaded) return PostHog;
  try {
    const posthogModule = await import("tauri-plugin-posthog-api");
    PostHog = posthogModule.PostHog;
    posthogLoaded = true;
    return PostHog;
  } catch (error) {
    // Plugin not available - that's okay
    posthogLoaded = true; // Mark as loaded to avoid repeated attempts
    return null;
  }
};

/**
 * Event names for tracking
 */
export const ANALYTICS_EVENTS = {
  // App Lifecycle
  APP_STARTED: "app_started",
  // License Events
  GET_LICENSE: "get_license",
} as const;

/**
 * Capture an analytics event
 */
export const captureEvent = async (
  eventName: string,
  properties?: Record<string, any>
) => {
  try {
    const posthog = await loadPostHog();
    if (posthog && typeof posthog.capture === 'function') {
      await posthog.capture(eventName, properties || {});
    }
  } catch (error: any) {
    // Silently fail - we don't want analytics to break the app
    // Ignore "not initialized", "no token", or "plugin not found" errors
    const errorMsg = error?.message || String(error);
    if (!errorMsg.includes('not initialized') && 
        !errorMsg.includes('token') && 
        !errorMsg.includes('plugin not found') &&
        !errorMsg.includes('not allowed')) {
      console.debug("Analytics event failed:", eventName, error);
    }
  }
};

/**
 * Track app initialization
 */
export const trackAppStart = async (appVersion: string, instanceId: string) => {
  await captureEvent(ANALYTICS_EVENTS.APP_STARTED, {
    app_version: appVersion,
    platform: navigator.platform,
    instance_id: instanceId,
  });
};
