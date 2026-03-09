import { useEffect, useState, useCallback } from "react";
import { useTitles, useSystemAudio, useInactivity, useVoiceActivation } from "@/hooks";
import { useApp as useAppContext } from "@/contexts";
import { listen } from "@tauri-apps/api/event";
import { safeLocalStorage, migrateLocalStorageToSQLite } from "@/lib";
import { getShortcutsConfig, getVoiceActivationState, updateLogoPosition } from "@/lib/storage";
import { invoke } from "@tauri-apps/api/core";

const LOGO_SIZE = 80;
const FULL_WIDTH = 1200;
const FULL_HEIGHT = 800;
const INACTIVITY_TIMEOUT_MS = 25000;

export const useApp = () => {
  const systemAudio = useSystemAudio();
  const [isHidden, setIsHidden] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);

  const { voiceActivation: voiceActivationState } = useAppContext() as any;
  const autoCollapseEnabled = voiceActivationState?.autoCollapseEnabled ?? true;
  const voiceEnabled = voiceActivationState?.enabled ?? true;
  const customPhrase = voiceActivationState?.customPhrase ?? "";

  const handleCollapse = useCallback(async () => {
    try {
      await invoke("force_show_window");
      await invoke("set_always_on_top", { enabled: true });
      const [x, y] = await invoke<[number, number]>("get_logo_position_clamped", {
        width: LOGO_SIZE,
        height: LOGO_SIZE,
        saved_x: null,
        saved_y: null,
      });
      await invoke("set_window_size", { width: LOGO_SIZE, height: LOGO_SIZE });
      await invoke("set_window_position", { x, y });
      updateLogoPosition(x, y);
      await new Promise((r) => setTimeout(r, 150));
      await invoke("force_show_window");
      setIsCollapsed(true);
    } catch (e) {
      console.error("Failed to collapse:", e);
    }
  }, []);

  const handleExpand = useCallback(async () => {
    try {
      if (isCollapsed) {
        const [x, y] = await invoke<[number, number]>("get_window_position");
        updateLogoPosition(x, y);
      }
      await invoke("set_window_size", { width: FULL_WIDTH, height: FULL_HEIGHT });
      await invoke("center_main_window");
      await invoke("set_always_on_top", { enabled: false });
      setIsCollapsed(false);
      invoke("force_show_window").catch(() => {});
    } catch (e) {
      console.error("Failed to expand:", e);
      setIsCollapsed(false);
    }
  }, [isCollapsed]);

  useInactivity(handleCollapse, {
    timeoutMs: INACTIVITY_TIMEOUT_MS,
    enabled:
      autoCollapseEnabled &&
      !isCollapsed &&
      !isHidden &&
      systemAudio?.capturing !== true,
  });

  useVoiceActivation(handleExpand, {
    enabled: isCollapsed && voiceEnabled,
    customPhrase: customPhrase || undefined,
  });

  // Initialize title management
  useTitles();

  // Initialize shortcuts from localStorage on app startup
  useEffect(() => {
    const initializeShortcuts = async () => {
      try {
        // Safety: ensure app window is visible on startup even if a prior toggle desynced state.
        setIsHidden(false);
        await invoke("force_show_window").catch(() => {});

        const config = getShortcutsConfig();
        await invoke("update_shortcuts", { config });
      } catch (error) {
        console.error("Failed to initialize shortcuts:", error);
      }
    };

    initializeShortcuts();
  }, []);

  // Migrate localStorage chat history to SQLite on app startup
  useEffect(() => {
    const runMigration = async () => {
      try {
        // Early exit: Check if migration already completed
        const migrationKey = "chat_history_migrated_to_sqlite";
        const alreadyMigrated =
          safeLocalStorage.getItem(migrationKey) === "true";

        if (alreadyMigrated) {
          return; // Migration already complete, skip
        }

        const result = await migrateLocalStorageToSQLite();

        if (result.success) {
          if (result.migratedCount > 0) {
            console.log(
              `Successfully migrated ${result.migratedCount} conversations to SQLite`
            );
          }
        } else if (result.error) {
          // Migration failed - log error
          console.error("Migration error:", result.error);
        }
      } catch (error) {
        // Critical error during migration
        console.error("Critical migration failure:", error);
      }
    };
    runMigration();
  }, []);

  const handleSelectConversation = (conversation: any) => {
    // useCompletion will fetch the full conversation from SQLite by id
    window.dispatchEvent(
      new CustomEvent("conversationSelected", {
        detail: { id: conversation.id },
      })
    );
  };

  const handleNewConversation = () => {
    // Trigger new conversation event
    window.dispatchEvent(new CustomEvent("newConversation"));
  };

  // WINDOWS HIDE/SHOW TOGGLE WINDOW WORKAROUND FOR SHORTCUTS
  useEffect(() => {
    const unlistenPromise = listen<boolean>(
      "toggle-window-visibility",
      (event) => {
        const platform = navigator.platform.toLowerCase();
        if (typeof event.payload === "boolean" && platform.includes("win")) {
          // Rust emits true = window hidden, false = window visible
          setIsHidden(event.payload);
          // find popover open and close it
          const popover = document.getElementById("popover-content");
          // set display to none, change data-state to closed
          if (popover) {
            popover.style.setProperty("display", "none", "important");
            // update the data-state to closed
            popover.setAttribute("data-state", "closed");

            // Also find and update the popover trigger's data-state
            const popoverTriggers = document.querySelectorAll(
              '[data-slot="popover-trigger"]'
            );
            popoverTriggers.forEach((trigger) => {
              trigger.setAttribute("data-state", "closed");
            });
          }
        }
      }
    );

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  return {
    isHidden,
    setIsHidden,
    isCollapsed,
    handleExpand,
    handleSelectConversation,
    handleNewConversation,
    systemAudio,
  };
};
