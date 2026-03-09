import { useCallback, useRef } from "react";

interface FloatingLogoProps {
  onExpand: () => void;
}

const CLICK_MOVE_THRESHOLD = 5;

export const FloatingLogo = ({ onExpand }: FloatingLogoProps) => {
  const pointerDownRef = useRef<{ x: number; y: number } | null>(null);

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    pointerDownRef.current = { x: e.clientX, y: e.clientY };
  }, []);

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      const down = pointerDownRef.current;
      pointerDownRef.current = null;
      if (!down) return;
      const dx = e.clientX - down.x;
      const dy = e.clientY - down.y;
      if (dx * dx + dy * dy <= CLICK_MOVE_THRESHOLD * CLICK_MOVE_THRESHOLD) {
        onExpand();
      }
    },
    [onExpand]
  );

  return (
    <div
      className="w-full h-full flex items-center justify-center select-none bg-transparent pointer-events-none"
      data-tauri-drag-region
    >
      <div
        className="flex items-center justify-center cursor-pointer w-20 h-20 rounded-full hover:bg-white/10 transition-colors pointer-events-auto"
        style={{ WebkitAppRegion: "no-drag", appRegion: "no-drag" } as React.CSSProperties}
        onClick={onExpand}
        onPointerDown={handlePointerDown}
        onPointerUp={handlePointerUp}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onExpand();
          }
        }}
        aria-label="Click to expand Ghost"
      >
        <div
          className="w-16 h-16 bg-primary pointer-events-none"
          style={{
            maskImage: "url(/ghost_logo.png)",
            WebkitMaskImage: "url(/ghost_logo.png)",
            maskSize: "contain",
            WebkitMaskSize: "contain",
            maskRepeat: "no-repeat",
            WebkitMaskRepeat: "no-repeat",
            maskPosition: "center",
            WebkitMaskPosition: "center",
            filter: "brightness(1.3) saturate(1.6) contrast(1.4) drop-shadow(0 0 8px rgba(255, 154, 139, 0.6))",
          }}
          role="img"
          aria-label="Ghost"
        />
      </div>
    </div>
  );
};
