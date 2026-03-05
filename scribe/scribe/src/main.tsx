import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Overlay from "./components/Overlay";
import { AppProvider, ThemeProvider } from "./contexts";
import "./global.css";
import { getCurrentWindow } from "@tauri-apps/api/window";

function init() {
  const root = document.getElementById("root");
  if (!root) return;

  let windowLabel = "main";
  try {
    const currentWindow = getCurrentWindow();
    windowLabel = currentWindow.label;
  } catch (e) {
    console.warn("getCurrentWindow failed, using main:", e);
  }

  if (windowLabel === "capture-overlay") {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <Overlay />
      </React.StrictMode>
    );
  } else {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <ThemeProvider>
          <AppProvider>
            <App />
          </AppProvider>
        </ThemeProvider>
      </React.StrictMode>
    );
  }
}

init();
