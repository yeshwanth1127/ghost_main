import { PowerIcon } from "lucide-react";
import { useApp } from "@/contexts";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export const Disclaimer = () => {
  useApp();
  const [displayText, setDisplayText] = useState("");
  const [isTyping, setIsTyping] = useState(true);
  const fullText = "EXORA";

  useEffect(() => {
    let timeoutId: NodeJS.Timeout;

    const animateText = () => {
      const currentLength = displayText.length;
      
      if (isTyping) {
        // Typing phase
        if (currentLength < fullText.length) {
          setDisplayText(fullText.substring(0, currentLength + 1));
        } else {
          // Wait before switching to erasing
          setTimeout(() => setIsTyping(false), 1000);
        }
      } else {
        // Erasing phase
        if (currentLength > 0) {
          setDisplayText(fullText.substring(0, currentLength - 1));
        } else {
          // Wait before typing again
          setTimeout(() => setIsTyping(true), 500);
        }
      }
    };

    timeoutId = setTimeout(animateText, 150);

    return () => clearTimeout(timeoutId);
  }, [displayText, isTyping, fullText]);

  return (
    <div className="flex items-center justify-between py-4 px-4">
      {/* Left: bug link */}
      <div className="flex flex-row items-center gap-2">
        <a
          href="https://exora.solutions/join#contact"
          target="_blank"
          rel="noopener noreferrer"
          className="text-muted-foreground hover:text-primary transition-colors text-sm font-medium"
        >
          Report a bug
        </a>
      </div>
      {/* Right: typing animation and quit button */}
      <div className="flex items-center gap-4">
        <div
          className="text-xs font-medium text-purple-600"
          style={{ fontFamily: '"Press Start 2P", monospace' }}
        >
          {displayText}
        </div>
        <div
          onClick={async () => {
            await invoke("exit_app");
          }}
          className="ml-2 text-muted-foreground hover:text-primary transition-colors cursor-pointer"
          title="Quit the application"
        >
          <PowerIcon className="w-5 h-5" />
        </div>
      </div>
    </div>
  );
};
