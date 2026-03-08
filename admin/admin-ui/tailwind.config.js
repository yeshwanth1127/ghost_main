/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ghost: {
          bg: "#0f172a",
          surface: "#1e293b",
          border: "#334155",
          muted: "#94a3b8",
          text: "#e2e8f0",
          accent: "#3b82f6",
          error: "#f87171",
        },
      },
      fontFamily: {
        sans: ["system-ui", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
      },
    },
  },
  plugins: [],
};
