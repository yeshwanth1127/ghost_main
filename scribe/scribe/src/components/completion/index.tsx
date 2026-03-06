import {
  Popover,
  PopoverContent,
  PopoverTrigger,
  PopoverAnchor,
} from "@/components";
import { useCompletion } from "@/hooks";
import { Input } from "./Input";
import { AudioGroup } from "./AudioGroup";
import { MediaGroup } from "./MediaGroup";
import { ResponsePanel, ResponsePanelContent } from "./ResponsePanel";
import { useSystemAudioType } from "@/hooks";

export const Completion = ({
  isHidden,
  systemAudio,
}: {
  isHidden: boolean;
  systemAudio?: useSystemAudioType;
}) => {
  const completion = useCompletion();
  const { isPopoverOpen, isLoading, keepEngaged, reset, response } = completion;

  console.log("[Completion] Render state:", {
    isPopoverOpen,
    isLoading,
    responseLength: response?.length || 0,
    responsePreview: response?.slice(0, 50),
  });

  return (
    <Popover
      open={isPopoverOpen}
      onOpenChange={(open) => {
        if (!open && !isLoading && !keepEngaged) {
          reset();
        }
      }}
    >
      <PopoverAnchor asChild>
        <div className="flex w-full items-center gap-2 min-w-0">
          {/* Left logo beside input */}
          <img
            src={"/ghost_logo.png"}
            alt="Ghost"
            className="shrink-0 select-none pointer-events-none rounded-md w-9 h-8"
            style={{
              opacity: 1,
              filter: "brightness(1.4) contrast(1.2) saturate(1.1)",
            }}
          />
          {/* Input expands to fill space */}
          <PopoverTrigger asChild className="!border-none !bg-transparent flex-1 min-w-0">
            <div className="flex-1 min-w-0 flex">
              <Input {...completion} isHidden={isHidden} />
            </div>
          </PopoverTrigger>
          {/* Compact button groups */}
          <AudioGroup completion={completion} systemAudio={systemAudio} />
          <MediaGroup completion={completion} />
        </div>
      </PopoverAnchor>

      <PopoverContent
        align="start"
        side="bottom"
        className="w-[min(896px,calc(100vw-2rem))] max-w-4xl max-h-[calc(100vh-12rem)] min-h-[240px] p-0 pt-2 border shadow-lg overflow-hidden"
        sideOffset={8}
        collisionPadding={16}
        avoidCollisions={true}
      >
        <ResponsePanel {...completion} />
        <ResponsePanelContent {...completion} />
      </PopoverContent>
    </Popover>
  );
};
