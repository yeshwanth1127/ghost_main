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
          <div
            className="shrink-0 select-none pointer-events-none w-12 h-10 bg-primary"
            style={{
              maskImage: "url(/ghost_logo.png)",
              WebkitMaskImage: "url(/ghost_logo.png)",
              maskSize: "contain",
              WebkitMaskSize: "contain",
              maskRepeat: "no-repeat",
              WebkitMaskRepeat: "no-repeat",
              maskPosition: "center",
              WebkitMaskPosition: "center",
              filter: "brightness(1.3) saturate(1.6) contrast(1.4) drop-shadow(0 0 6px rgba(255, 154, 139, 0.5))",
            }}
            role="img"
            aria-label="Ghost"
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
        sideOffset={25}
        collisionPadding={16}
        avoidCollisions={true}
        style={{
          transform: 'translateX(-15px)',
        }}
      >
        <ResponsePanel {...completion} />
        <ResponsePanelContent {...completion} />
      </PopoverContent>
    </Popover>
  );
};
