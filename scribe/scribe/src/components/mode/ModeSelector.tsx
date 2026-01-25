import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";

type AppMode = "chat" | "agent" | null;

interface ModeSelectorProps {
  onModeSelect: (mode: "chat" | "agent") => void;
}

export const ModeSelector = ({ onModeSelect }: ModeSelectorProps) => {
  const [isOnline, setIsOnline] = useState<boolean>(navigator.onLine);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);

  useEffect(() => {
    const connection = (navigator as any).connection;

    const updateConnection = () => {
      if (typeof navigator.onLine === "boolean") {
        setIsOnline(navigator.onLine);
      }
      if (connection && typeof connection.rtt === "number") {
        setLatencyMs(connection.rtt);
      } else {
        setLatencyMs(null);
      }
    };

    updateConnection();

    window.addEventListener("online", updateConnection);
    window.addEventListener("offline", updateConnection);

    if (connection && typeof connection.addEventListener === "function") {
      connection.addEventListener("change", updateConnection);
    }

    const intervalId = window.setInterval(updateConnection, 5000);

    return () => {
      window.removeEventListener("online", updateConnection);
      window.removeEventListener("offline", updateConnection);
      if (connection && typeof connection.removeEventListener === "function") {
        connection.removeEventListener("change", updateConnection);
      }
      window.clearInterval(intervalId);
    };
  }, []);

  return (
    <div className="w-full flex flex-col items-center justify-center gap-6 p-8">
      <div className="text-center">
        <h2 className="text-3xl font-semibold tracking-tight mb-2">
          Choose Your Mode
        </h2>
        <p className="text-sm text-muted-foreground/80">
          Select how you want to interact with Ghost
        </p>
      </div>

      <div className="flex flex-row gap-6 w-full max-w-3xl">
        <Card
          className="group flex-1 p-6 cursor-pointer transition-all border border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/20 hover:shadow-[0_8px_30px_rgba(0,0,0,0.25)] backdrop-blur-xl rounded-2xl"
          onClick={() => onModeSelect("chat")}
        >
          <div className="flex flex-col items-center gap-3">
            <div className="h-12 w-12 rounded-full bg-white/10 border border-white/10 flex items-center justify-center text-2xl shadow-inner">
              💬
            </div>
            <h3 className="text-lg font-semibold">Chat Mode</h3>
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
          className="group flex-1 p-6 cursor-pointer transition-all border border-white/10 bg-white/5 hover:bg-white/10 hover:border-white/20 hover:shadow-[0_8px_30px_rgba(0,0,0,0.25)] backdrop-blur-xl rounded-2xl"
          onClick={() => onModeSelect("agent")}
        >
          <div className="flex flex-col items-center gap-3">
            <div className="h-12 w-12 rounded-full bg-white/10 border border-white/10 flex items-center justify-center text-2xl shadow-inner">
              🤖
            </div>
            <h3 className="text-lg font-semibold">Agent Mode</h3>
            <p className="text-sm text-muted-foreground/80 text-center">
              Autonomous agent that executes tasks
            </p>
          </div>
        </Card>
      </div>

      <div className="w-full max-w-3xl flex items-start justify-start pt-1">
        <div className="flex items-start gap-2 px-3 py-2 rounded-xl border border-white/10 bg-white/5 backdrop-blur-xl">
          <span className="mt-1 h-2.5 w-2.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.9)]" />
          <div className="text-xs text-muted-foreground/80">
            <div className="text-emerald-400 font-medium">
              {isOnline ? "Connected to Internet" : "Offline"}
            </div>
            <div>
              {latencyMs !== null ? `Latency: ${latencyMs} ms` : "Latency: --"}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
