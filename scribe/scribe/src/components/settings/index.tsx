import { APP_ENDPOINT } from "@/config";
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
import { AudioSelection } from "./AudioSelection";
import { AutostartToggle } from "./AutostartToggle";
import { AppIconToggle } from "./AppIconToggle";
import { AlwaysOnTopToggle } from "./AlwaysOnTopToggle";
import { AIProviders } from "./ai-configs";
import { STTProviders } from "./stt-configs";
import { DeleteChats } from "./DeleteChats";
import { ScribeApiSetup } from "./ScribeApiSetup";
import Theme from "./Theme";
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
        const response = await fetch(`${APP_ENDPOINT}/api/v1/auth/get-user`, {
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

  useEffect(() => {
    const handleWindowBlur = () => {
      settings?.setIsPopoverOpen(false);
    };

    window.addEventListener("blur", handleWindowBlur);
    return () => window.removeEventListener("blur", handleWindowBlur);
  }, [settings]);

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
        align="start"
        side="bottom"
        className="select-none w-full max-w-4xl p-0 border border-input/50 rounded-lg overflow-hidden pointer-events-auto flex flex-col"
        sideOffset={34}
        collisionPadding={16}
        avoidCollisions={true}
        style={{
          transform: 'translateX(-135px)',
          maxHeight: 'calc(100vh - 40px)',
        }}
      >
        <ScrollArea className="h-[calc(100vh-13rem)]">
          <div className="flex min-h-full">
            <div className="px-4 py-6 space-y-6 w-full flex flex-col justify-center pointer-events-auto">
            {/* Usage & Billing Dashboard */}
            <UsageDashboard userId={userId} />

            {/* Ghost API Setup */}
            <ScribeApiSetup />

            {/* Provider Selection */}
            <AIProviders {...settings} />

            {/* STT Providers */}
            <STTProviders {...settings} />

            {/* System Prompt */}
            <SystemPrompt {...settings} />

            {/* Theme */}
            <Theme />

            {/* Mode Toggle */}
            <ModeToggle />

            {/* Audio Selection */}
            <AudioSelection />

            {/* Autostart Toggle */}
            <AutostartToggle />

            {/* App Icon Toggle */}
            <AppIconToggle />

            {/* Always On Top Toggle */}
            <AlwaysOnTopToggle />

            {/* Delete Chat History */}
            <DeleteChats {...settings} />
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
