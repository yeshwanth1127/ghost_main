import { useSettings } from "@/hooks";
import { SettingsIcon } from "lucide-react";
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

export const Settings = () => {
  const settings = useSettings();

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
        className="select-none w-screen p-0 border border-input/50 rounded-lg overflow-hidden"
        sideOffset={8}
      >
        <ScrollArea className="h-[calc(100vh-9rem)]">
          <div className="flex min-h-full">
            <div className="p-6 space-y-6 w-full flex flex-col justify-center">
            {/* Settings Navigation */}
            <SettingsNavigation />

            {/* Scribe API Setup */}
            <ScribeApiSetup />

            {/* Provider Selection */}
            <AIProviders {...settings} />

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
