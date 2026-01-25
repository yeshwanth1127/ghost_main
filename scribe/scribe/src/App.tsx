import {
  Card,
  Settings,
  Updater,
  DragButton,
  CustomCursor,
  Completion,
  ChatHistory,
  AudioVisualizer,
  StatusIndicator,
  ModeSelector,
  Button,
  AgentView,
} from "@/components";
import { useApp } from "@/hooks";
import { useApp as useAppContext } from "@/contexts";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCustomizableState, updateAppMode, AppMode } from "@/lib/storage";

// Initialize mode from saved preference
const getInitialMode = (): "chat" | "agent" | null => {
  try {
    const savedState = getCustomizableState();
    return savedState.mode?.type ?? null;
  } catch {
    return null;
  }
};

const App = () => {
  const [appMode, setAppMode] = useState<"chat" | "agent" | null>(getInitialMode);
  const {
    isHidden,
    systemAudio,
    handleSelectConversation,
    handleNewConversation,
  } = useApp();
  const { customizable, trialExpired } = useAppContext() as any;
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Listen for mode changes from settings
  useEffect(() => {
    const handleModeChange = (event: CustomEvent<{ mode: AppMode }>) => {
      setAppMode(event.detail.mode);
    };

    window.addEventListener("mode-change", handleModeChange as EventListener);
    return () => {
      window.removeEventListener("mode-change", handleModeChange as EventListener);
    };
  }, []);

  // Save mode when it changes from settings or mode selector
  // Note: We use a ref to track if this is the initial mount
  const isInitialMount = useRef(true);
  useEffect(() => {
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return; // Don't save on initial mount
    }
    // Save mode when it changes
    updateAppMode(appMode);
  }, [appMode]);

  useEffect(() => {
    if (appMode !== "agent") {
      invoke("cancel_all_runs").catch(() => {});
    }
  }, [appMode]);

  useEffect(() => {
    // Set a stable window size once on mount to avoid flicker/clipping
    invoke("set_window_size", { width: 1200, height: 800 }).catch(() => {
      invoke("set_window_height", { height: 800 }).catch(() => {});
    });
  }, []);

  // Show mode selector if no mode selected
  if (appMode === null) {
    return (
      <div className="w-screen h-screen flex overflow-hidden justify-center items-center px-3 py-3">
        <Card className="w-full max-w-5xl flex flex-col items-center gap-3 px-4 py-6 overflow-visible rounded-2xl border border-input/50 bg-background/95 shadow-xl backdrop-blur">
          <ModeSelector
            onModeSelect={(mode) => {
              setAppMode(mode);
              updateAppMode(mode);
            }}
          />
        </Card>
      </div>
    );
  }

  // Show agent mode UI
  if (appMode === "agent") {
    return (
      <div className="w-screen h-screen flex overflow-hidden justify-center items-start px-3 py-3">
        <Card className="w-full max-w-5xl max-h-[calc(100vh-3rem)] flex flex-col items-center gap-3 px-4 py-4 min-h-[220px] overflow-hidden rounded-2xl border border-input/50 bg-background/95 shadow-xl backdrop-blur">
          <div className="w-full flex flex-row items-center justify-between mb-2">
            <h2 className="text-lg font-semibold">Agent Mode</h2>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setAppMode(null);
                updateAppMode(null);
              }}
            >
              Switch Mode
            </Button>
          </div>
          <div className="w-full flex-1 overflow-auto">
            <AgentView />
          </div>
        </Card>
      </div>
    );
  }

  // Show chat mode (classic UI)
  return (
    <div
      className={`w-screen h-screen flex overflow-visible justify-center items-start px-3 py-3 ${
        isHidden ? "hidden pointer-events-none" : ""
      }`}
    >
      <Card
        ref={containerRef as any}
        className="w-full flex flex-row items-center gap-3 px-4 py-3 min-h-[96px] overflow-visible rounded-2xl border border-input/50 bg-background/95 shadow-xl backdrop-blur"
      >
        {trialExpired ? (
          <div className="absolute top-1 left-1 right-1 mx-2 px-3 py-1 text-xs rounded bg-amber-100 text-amber-700 border border-amber-200">
            Trial expired. Please upgrade to continue.
          </div>
        ) : null}
        {systemAudio?.capturing ? (
          <div className="flex flex-row items-center gap-2 justify-between w-full">
            <div className="flex flex-1 items-center gap-2">
              <AudioVisualizer
                stream={systemAudio?.stream}
                isRecording={systemAudio?.capturing}
              />
            </div>
            <div className="flex !w-fit items-center gap-2">
              <StatusIndicator
                setupRequired={systemAudio.setupRequired}
                error={systemAudio.error}
                isProcessing={systemAudio.isProcessing}
                isAIProcessing={systemAudio.isAIProcessing}
                capturing={systemAudio.capturing}
              />
            </div>
          </div>
        ) : null}

        <div
          className={`${
            systemAudio?.capturing
              ? "hidden w-full fade-out transition-all duration-300"
              : "w-full flex flex-row gap-2 items-center"
          }`}
        >
          <Completion isHidden={isHidden} systemAudio={systemAudio} />
          <ChatHistory
            onSelectConversation={handleSelectConversation}
            onNewConversation={handleNewConversation}
            currentConversationId={null}
          />
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              setAppMode(null);
              updateAppMode(null);
            }}
          >
            Switch Mode
          </Button>
          <Settings />
        </div>

        <Updater />
        <DragButton />
      </Card>
      {customizable.cursor.type === "invisible" ? <CustomCursor /> : null}
    </div>
  );
};

export default App;
