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
import { AIProviders } from "./ai-configs";
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
        className="select-none w-[min(896px,calc(100vw-2rem))] max-w-4xl min-h-[70vh] p-0 border border-input/50 rounded-xl overflow-hidden shadow-xl"
        sideOffset={12}
        collisionPadding={16}
        avoidCollisions={true}
      >
        <ScrollArea className="h-[calc(100vh-10rem)] min-h-[60vh]">
          <div className="flex min-h-full">
            <div className="p-6 w-full flex flex-col divide-y divide-input/30">
            {/* Settings Navigation */}
            <section className="pt-0 pb-5">
              <SettingsNavigation />
            </section>

            {/* Usage & Billing Dashboard */}
            <section className="py-5">
              <UsageDashboard userId={userId} />
            </section>

            {/* Ghost API Setup */}
            <section className="py-5">
              <ScribeApiSetup />
            </section>

            {/* Provider Selection */}
            <section className="py-5">
              <AIProviders {...settings} />
            </section>

            {/* STT Providers */}
            <section className="py-5">
              <STTProviders {...settings} />
            </section>

            {/* System Prompt */}
            <section className="py-5">
              <SystemPrompt {...settings} />
            </section>

            {/* Theme */}
            <section className="py-5">
              <Theme />
            </section>

            {/* Screenshot Configs */}
            <section className="py-5">
              <ScreenshotConfigs {...settings} />
            </section>

            {/* Cursor Selection */}
            <section className="py-5">
              <CursorSelection />
            </section>

            {/* Mode Toggle */}
            <section className="py-5">
              <ModeToggle />
            </section>

            {/* Keyboard Shortcuts */}
            <section className="py-5">
              <ShortcutManager />
            </section>

            {/* Audio Selection */}
            <section className="py-5">
              <AudioSelection />
            </section>

            {/* Autostart Toggle */}
            <section className="py-5">
              <AutostartToggle />
            </section>

            {/* App Icon Toggle */}
            <section className="py-5">
              <AppIconToggle />
            </section>

            {/* Always On Top Toggle */}
            <section className="py-5">
              <AlwaysOnTopToggle />
            </section>

            {/* Title Toggle */}
            <section className="py-5">
              <TitleToggle />
            </section>

            {/* Delete Chat History */}
            <section className="py-5">
              <DeleteChats {...settings} />
            </section>

            {/* Apply for Leave */}
            <section className="py-5 pb-0">
              <ApplyForLeave />
            </section>
            </div>
          </div>
        </ScrollArea>

        {/* Footer always visible at the bottom */}
        <div className="border-t border-input/30 px-6 py-4 bg-background">
          <Disclaimer />
        </div>
      </PopoverContent>
    </Popover>
  );
};
