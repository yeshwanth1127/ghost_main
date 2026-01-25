import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue, Header } from "@/components";
import { useApp } from "@/contexts";
import { updateAppMode, AppMode } from "@/lib/storage";
import { MessageSquare, Bot } from "lucide-react";

interface ModeToggleProps {
  className?: string;
  onModeChange?: (mode: AppMode) => void;
}

export const ModeToggle = ({ className, onModeChange }: ModeToggleProps) => {
  const { customizable, loadData } = useApp();
  const currentMode = customizable?.mode?.type ?? null;

  const handleModeChange = (value: string) => {
    const newMode: AppMode = value === "null" ? null : (value as "chat" | "agent");
    updateAppMode(newMode);
    // Refresh the context to reflect the change
    if (loadData) {
      loadData();
    }
    if (onModeChange) {
      onModeChange(newMode);
    }
    // Trigger a custom event to notify App.tsx
    window.dispatchEvent(new CustomEvent("mode-change", { detail: { mode: newMode } }));
  };

  return (
    <div id="mode" className={`space-y-2 ${className}`}>
      <Header
        title="App Mode"
        description="Choose between Chat and Agent mode"
        isMainTitle
        rightSlot={
          <Select
            value={currentMode === null ? "null" : currentMode}
            onValueChange={handleModeChange}
          >
            <SelectTrigger>
              <SelectValue placeholder="Select mode" />
            </SelectTrigger>
            <SelectContent position="popper" align="end">
              <SelectItem value="null">
                <div className="flex items-center gap-2">
                  <span>Ask on Start</span>
                </div>
              </SelectItem>
              <SelectItem value="chat">
                <div className="flex items-center gap-2">
                  <MessageSquare className="size-3" />
                  <span>Chat Mode</span>
                </div>
              </SelectItem>
              <SelectItem value="agent">
                <div className="flex items-center gap-2">
                  <Bot className="size-3" />
                  <span>Agent Mode</span>
                </div>
              </SelectItem>
            </SelectContent>
          </Select>
        }
      />
      <p className="text-xs text-muted-foreground">
        {currentMode === null
          ? "You'll be asked to choose a mode when the app starts"
          : currentMode === "chat"
          ? "Chat mode: Interactive conversation with AI assistant"
          : "Agent mode: Autonomous agent that executes tasks"}
      </p>
    </div>
  );
};
