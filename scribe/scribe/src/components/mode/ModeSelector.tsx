import { useEffect, useState, useRef } from "react";
import { Card } from "@/components/ui/card";
import { MessageSquare, Bot } from "lucide-react";
import ShinyText from "@/components/ShinyText/ShinyText";

type AppMode = "chat" | "agent" | null;

interface ModeSelectorProps {
  onModeSelect: (mode: "chat" | "agent") => void;
}

export const ModeSelector = ({ onModeSelect }: ModeSelectorProps) => {
  const [isOnline, setIsOnline] = useState<boolean>(navigator.onLine);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const latencyIntervalRef = useRef<number | null>(null);

  useEffect(() => {
    const connection = (navigator as any).connection || (navigator as any).mozConnection || (navigator as any).webkitConnection;

    const measureLatency = () => {
      if (!navigator.onLine) {
        setLatencyMs(null);
        return;
      }

      // Use NetworkInformation API (standard way to get network latency)
      if (connection && typeof connection.rtt === "number" && connection.rtt > 0) {
        setLatencyMs(connection.rtt);
      } else {
        setLatencyMs(null);
      }
    };

    const updateConnection = () => {
      if (typeof navigator.onLine === "boolean") {
        setIsOnline(navigator.onLine);
      }
    };

    const handleOnline = () => {
      updateConnection();
      measureLatency();
    };

    const handleOffline = () => {
      updateConnection();
      measureLatency();
    };

    const handleConnectionChange = () => {
      updateConnection();
      measureLatency();
    };

    updateConnection();
    measureLatency();

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    if (connection && typeof connection.addEventListener === "function") {
      connection.addEventListener("change", handleConnectionChange);
    }

    // Measure latency every 2 seconds for dynamic updates
    latencyIntervalRef.current = window.setInterval(measureLatency, 2000);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
      if (connection && typeof connection.removeEventListener === "function") {
        connection.removeEventListener("change", handleConnectionChange);
      }
      if (latencyIntervalRef.current) {
        window.clearInterval(latencyIntervalRef.current);
      }
    };
  }, []);

  return (
    <div className="w-full flex flex-col items-center justify-center gap-6 p-8">
      <div className="text-center">
        <h2 className="text-3xl font-semibold tracking-tight mb-2 text-purple-500">
          Choose Your Mode
        </h2>
        <p className="text-sm text-muted-foreground/80">
          Select how you want to interact with Ghost
        </p>
      </div>

      <div className="flex flex-row gap-6 w-full max-w-2xl">
        <Card
          className="group flex-1 p-5 cursor-pointer transition-all border border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/20 hover:shadow-[0_8px_30px_rgba(0,0,0,0.25)] backdrop-blur-xl rounded-2xl"
          onClick={() => onModeSelect("chat")}
        >
          <div className="flex flex-col items-center gap-3">
            <div className="h-10 w-10 rounded-full bg-white/10 border border-white/10 flex items-center justify-center">
              <MessageSquare className="h-5 w-5 text-white/80" />
            </div>
            <h3 className="text-lg font-semibold" style={{ fontFamily: '"Bitcount Single", monospace' }}>
              <ShinyText
                text="Chat Mode"
                speed={2}
                delay={0}
                color="#b5b5b5"
                shineColor="#ffffff"
                spread={120}
                direction="left"
                yoyo={false}
                pauseOnHover={false}
                disabled={false}
              />
            </h3>
            <p className="text-sm text-muted-foreground/80 text-center">
              Interactive conversation with AI assistant
            </p>
          </div>
        </Card>

        <div
          aria-hidden="true"
          className="w-px self-stretch bg-gradient-to-b from-transparent via-white/15 to-transparent"
        />

        <Card
          className="group flex-1 p-5 cursor-pointer transition-all border border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/20 hover:shadow-[0_8px_30px_rgba(0,0,0,0.25)] backdrop-blur-xl rounded-2xl"
          onClick={() => onModeSelect("agent")}
        >
          <div className="flex flex-col items-center gap-3">
            <div className="h-10 w-10 rounded-full bg-white/10 border border-white/10 flex items-center justify-center">
              <Bot className="h-5 w-5 text-white/80" />
            </div>
            <h3 className="text-lg font-semibold" style={{ fontFamily: '"Bitcount Single", monospace' }}>
              <ShinyText
                text="Agent Mode"
                speed={2}
                delay={0}
                color="#b5b5b5"
                shineColor="#ffffff"
                spread={120}
                direction="left"
                yoyo={false}
                pauseOnHover={false}
                disabled={false}
              />
            </h3>
            <p className="text-sm text-muted-foreground/80 text-center">
              Autonomous agent that executes tasks
            </p>
          </div>
        </Card>
      </div>

      <div className="w-full max-w-2xl flex items-start justify-start pt-1">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-emerald-500" />
          <div className="text-xs text-muted-foreground/80">
            <span className="text-emerald-400 font-medium">
              {isOnline ? "Connected to Internet" : "Offline"}
            </span>
            {isOnline && (
              <span className="ml-2">
                {latencyMs !== null ? `Latency: ${latencyMs} ms` : "Latency: --"}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
