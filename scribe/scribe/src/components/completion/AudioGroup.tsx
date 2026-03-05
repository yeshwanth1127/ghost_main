import { SystemAudio } from "../speech";
import { Audio } from "./Audio";
import { UseCompletionReturn } from "@/types";
import { useSystemAudioType } from "@/hooks";

export const AudioGroup = ({ 
  completion,
  systemAudio 
}: { 
  completion: UseCompletionReturn;
  systemAudio?: useSystemAudioType;
}) => {
  return (
    <div className="flex shrink-0 items-center border border-input rounded-lg overflow-hidden [&_button]:rounded-none [&_button]:border-0 [&_button]:shadow-none [&_button:hover]:bg-transparent [&>div:not(:last-child)]:border-r [&>div:not(:last-child)]:border-input/50">
      {systemAudio && (
        <div className="hover:bg-primary/10 transition-colors [&_button]:hover:bg-transparent">
          <SystemAudio {...systemAudio} />
        </div>
      )}
      <div className="hover:bg-primary/10 transition-colors [&_button]:hover:bg-transparent">
        <Audio {...completion} />
      </div>
    </div>
  );
};
