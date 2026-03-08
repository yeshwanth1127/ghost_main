export default function GhostIcon({ className = "w-12 h-12" }: { className?: string }) {
  const lines = Array.from({ length: 36 }).map((_, i) => {
    const angle = (i / 36) * Math.PI * 2;
    const wave = Math.sin(angle * 4) * 3 + Math.sin(angle * 7) * 2;
    const innerR = 6 + wave;
    const outerR = 26 + Math.sin(angle * 3) * 4;
    const x1 = 32 + Math.cos(angle) * innerR;
    const y1 = 32 + Math.sin(angle) * innerR;
    const x2 = 32 + Math.cos(angle) * outerR;
    const y2 = 32 + Math.sin(angle) * outerR;
    return { x1, y1, x2, y2 };
  });

  return (
    <svg
      viewBox="0 0 64 64"
      fill="none"
      className={className}
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle cx="32" cy="32" r="30" fill="#8a2be2" opacity="0.2" />
      {lines.map(({ x1, y1, x2, y2 }, i) => (
        <line
          key={i}
          x1={x1}
          y1={y1}
          x2={x2}
          y2={y2}
          stroke="#8a2be2"
          strokeWidth="1.2"
          strokeLinecap="round"
        />
      ))}
    </svg>
  );
}
