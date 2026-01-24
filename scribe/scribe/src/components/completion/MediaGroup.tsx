import { Screenshot } from "./Screenshot";
import { Files } from "./Files";
import { UseCompletionReturn } from "@/types";

export const MediaGroup = ({ 
  completion
}: { 
  completion: UseCompletionReturn;
}) => {
  return (
    <div className="flex items-center border border-input rounded-xl overflow-hidden px-1 bg-neutral-900 dark:bg-black [&_button]:rounded-none [&_button]:border-0 [&_button]:shadow-none [&_button:hover]:bg-transparent [&>div:not(:last-child)]:border-r [&>div:not(:last-child)]:border-input/50">
      <div className="flex-1 hover:bg-primary/10 transition-colors rounded-l-xl [&_button]:hover:bg-transparent">
        <Screenshot {...completion} />
      </div>
      <div className="flex-1 hover:bg-primary/10 transition-colors rounded-r-xl [&_button]:hover:bg-transparent">
        <Files {...completion} />
      </div>
    </div>
  );
};
