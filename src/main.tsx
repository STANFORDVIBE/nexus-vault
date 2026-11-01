import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { isTauri } from "@tauri-apps/api/core";
import { App } from "@/app/App";
import { initTheme } from "@/store/theme";
import "@/styles/globals.css";

async function start() {
  // Browser preview / e2e only: `import.meta.env.DEV` is false in release builds, so the
  // mock backend is tree-shaken away and never ships inside the app.
  if (import.meta.env.DEV && !isTauri()) {
    const { installMockBackend } = await import("@/dev/mockBackend");
    const seeded = new URLSearchParams(window.location.search).has("seed");
    installMockBackend(seeded ? { state: "locked", seed: true } : undefined);
  }

  initTheme();

  createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void start();
