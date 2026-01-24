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
} from "@/components";
import { useApp } from "@/hooks";
import { useApp as useAppContext } from "@/contexts";
import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
const App = () => {
  const {
    isHidden,
    systemAudio,
    handleSelectConversation,
    handleNewConversation,
  } = useApp();
  const { customizable, trialExpired } = useAppContext() as any;
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    // Set a stable window height once on mount to avoid flicker/clipping
    invoke("set_window_height", { height: 200 }).catch(() => {});
  }, []);

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
