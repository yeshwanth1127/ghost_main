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
    <div className="flex items-center border border-input rounded-xl overflow-hidden px-1 bg-neutral-900 dark:bg-black [&_button]:rounded-none [&_button]:border-0 [&_button]:shadow-none [&_button:hover]:bg-transparent [&>div:not(:last-child)]:border-r [&>div:not(:last-child)]:border-input/50">
      {systemAudio && (
        <div className="flex-1 hover:bg-primary/10 transition-colors rounded-l-xl [&_button]:hover:bg-transparent">
          <SystemAudio {...systemAudio} />
        </div>
      )}
      <div className={`flex-1 hover:bg-primary/10 transition-colors ${systemAudio ? 'rounded-r-xl' : 'rounded-l-xl rounded-r-xl'} [&_button]:hover:bg-transparent`}>
        <Audio {...completion} />
      </div>
    </div>
  );
};
