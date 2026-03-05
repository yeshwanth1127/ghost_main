import { Screenshot } from "./Screenshot";
import { Files } from "./Files";
import { UseCompletionReturn } from "@/types";

export const MediaGroup = ({
  completion,
}: {
  completion: UseCompletionReturn;
}) => {
  return (
    <div className="flex shrink-0 items-center border border-input rounded-lg overflow-hidden [&_button]:rounded-none [&_button]:border-0 [&_button]:shadow-none [&_button:hover]:bg-transparent [&>div:not(:last-child)]:border-r [&>div:not(:last-child)]:border-input/50">
      <div className="hover:bg-primary/10 transition-colors [&_button]:hover:bg-transparent">
        <Screenshot {...completion} />
      </div>
      <div className="hover:bg-primary/10 transition-colors [&_button]:hover:bg-transparent">
        <Files {...completion} />
      </div>
    </div>
  );
};
