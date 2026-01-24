import { useCompletion } from "@/hooks";
import { Input } from "./Input";
import { AudioGroup } from "./AudioGroup";
import { MediaGroup } from "./MediaGroup";
import { useSystemAudioType } from "@/hooks";

export const Completion = ({ 
  isHidden, 
  systemAudio 
}: { 
  isHidden: boolean;
  systemAudio?: useSystemAudioType;
}) => {
  const completion = useCompletion();

  return (
    <div className="flex flex-1 items-center gap-2 pb-1">
      {/* Left logo beside input */}
      <img
        src={"/ghost_logo.png"}
        alt="Ghost"
        className="shrink-0 select-none pointer-events-none rounded-md ml-2 mr-2"
        style={{ width: 42, height: 40, opacity: 1, filter: "brightness(1.4) contrast(1.2) saturate(1.1)" }}
      />
      {/* Input expands */}
      <div className="flex-1">
        <Input {...completion} isHidden={isHidden} />
      </div>
      {/* Grouped buttons on the right side of input */}
      <AudioGroup completion={completion} systemAudio={systemAudio} />
      <MediaGroup completion={completion} />
    </div>
  );
};
