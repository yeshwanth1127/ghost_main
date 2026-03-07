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
      className="w-full h-full flex items-center justify-center cursor-pointer select-none bg-transparent"
      data-tauri-drag-region
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
      <img
        src="/ghost_logo.png"
        alt="Ghost"
        className="w-14 h-14 object-contain pointer-events-none"
        draggable={false}
      />
    </div>
  );
};
