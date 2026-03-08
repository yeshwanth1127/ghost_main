import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import fs from "fs";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// @tauri-apps/api may be nested in plugins; resolve to first available
function findTauriApi(): string {
  const candidates = [
    path.join(__dirname, "node_modules/@tauri-apps/api"),
    path.join(__dirname, "node_modules/@tauri-apps/plugin-autostart/node_modules/@tauri-apps/api"),
    path.join(__dirname, "node_modules/tauri-plugin-keychain/node_modules/@tauri-apps/api"),
  ];
  for (const c of candidates) {
    if (fs.existsSync(path.join(c, "core.js"))) return c;
  }
  return candidates[0];
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@tauri-apps/api": findTauriApi(),
      "lucide-react": path.resolve(__dirname, "node_modules/lucide-react/dist/cjs/lucide-react.js"),
    },
  },
  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
