import { useSettings } from "@/hooks";
import { SettingsIcon } from "lucide-react";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  Button,
  ScrollArea,
} from "@/components";
import { Disclaimer } from "./Disclaimer";
import { SystemPrompt } from "./system-prompt";
import { ScreenshotConfigs } from "./ScreenshotConfigs";
import { AudioSelection } from "./AudioSelection";
import { AutostartToggle } from "./AutostartToggle";
import { AppIconToggle } from "./AppIconToggle";
import { AlwaysOnTopToggle } from "./AlwaysOnTopToggle";
import { TitleToggle } from "./TitleToggle";
import { STTProviders } from "./stt-configs";
import { DeleteChats } from "./DeleteChats";
import { ScribeApiSetup } from "./ScribeApiSetup";
import { ShortcutManager } from "./shortcuts";
import Theme from "./Theme";
import { SettingsNavigation } from "./SettingsNavigation";
import { CursorSelection } from "./Cursor";
import { ApplyForLeave } from "./ApplyForLeave";
import { ModeToggle } from "./ModeToggle";
import { UsageDashboard } from "./UsageDashboard";
import { VoiceActivation } from "./VoiceActivation";

export const Settings = () => {
  const settings = useSettings();
  const [userId, setUserId] = useState<string | undefined>(undefined);

  useEffect(() => {
    const fetchUserId = async () => {
      try {
        // Get stored license key (backend returns without Scribe_ prefix)
        const result: any = await invoke("secure_storage_get");
        
        const licenseKey = result?.license_key;
        console.log("🔍 License key from storage:", licenseKey ? "found" : "not found");
        
        if (!licenseKey) {
          console.log("❌ No license key found");
          setUserId(undefined);
          return;
        }

        // Fetch user_id from backend
        console.log("🌐 Fetching user_id from backend...");
        const response = await fetch('http://localhost:8083/api/v1/auth/get-user', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ license_key: licenseKey }),
        });

        if (response.ok) {
          const data = await response.json();
          console.log("✅ User ID fetched:", data.user_id);
          setUserId(data.user_id);
        } else {
          console.error("❌ Backend error:", response.status);
          setUserId(undefined);
        }
      } catch (err) {
        console.error('Failed to fetch user_id:', err);
        setUserId(undefined);
      }
    };

    // Fetch userId when settings popover opens
    if (settings?.isPopoverOpen) {
      console.log("📘 Settings popover opened, fetching user_id...");
      fetchUserId();
    }
  }, [settings?.isPopoverOpen]);

  useEffect(() => {
    console.log("📊 UserId state changed:", userId);
  }, [userId]);

  return (
    <Popover
      open={settings?.isPopoverOpen}
      onOpenChange={settings?.setIsPopoverOpen}
    >
      <PopoverTrigger asChild>
        <Button
          size="icon"
          aria-label="Open Settings"
          className="cursor-pointer"
          title="Open Settings"
        >
          <SettingsIcon className="h-4 w-4" />
        </Button>
      </PopoverTrigger>

      {/* Settings Panel */}
      <PopoverContent
        align="end"
        side="bottom"
        className="select-none w-[600px] max-w-[90vw] p-0 border border-input/50 rounded-lg overflow-hidden"
        sideOffset={8}
      >
        <ScrollArea className="h-[calc(100vh-9rem)]">
          <div className="flex min-h-full">
            <div className="p-6 space-y-6 w-full flex flex-col justify-center">
            {/* Settings Navigation */}
            <SettingsNavigation />

            {/* Usage & Billing Dashboard */}
            <UsageDashboard userId={userId} />

            {/* Ghost API Setup - includes the single model picker (Ghost supports X models) */}
            <ScribeApiSetup />

            {/* AI Providers section removed - app requires license; model picker is the only way to select */}

            {/* STT Providers */}
            <STTProviders {...settings} />

            {/* System Prompt */}
            <SystemPrompt {...settings} />

            {/* Theme */}
            <Theme />

            {/* Screenshot Configs */}
            <ScreenshotConfigs {...settings} />

            {/* Cursor Selection */}
            <CursorSelection />

            {/* Mode Toggle */}
            <ModeToggle />

            {/* Keyboard Shortcuts */}
            <ShortcutManager />

            {/* Audio Selection */}
            <AudioSelection />

            {/* Autostart Toggle */}
            <AutostartToggle />

            {/* App Icon Toggle */}
            <AppIconToggle />

            {/* Always On Top Toggle */}
            <AlwaysOnTopToggle />

            {/* Voice Activation */}
            <VoiceActivation />

            {/* Title Toggle */}
            <TitleToggle />

            {/* Delete Chat History */}
            <DeleteChats {...settings} />

            {/* Apply for Leave */}
            <ApplyForLeave />
            </div>
          </div>

          {/* Footer attribution removed as requested */}
        </ScrollArea>

        {/* Footer always visible at the bottom */}
        <div className="border-t border-input/50 px-6 py-4 bg-background">
          <Disclaimer />
        </div>
      </PopoverContent>
    </Popover>
  );
};
