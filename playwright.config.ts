import { defineConfig } from "@playwright/test";

// End-to-end UI tests in the system Microsoft Edge (same Chromium engine as WebView2),
// against the Vite dev server. Outside Tauri the app talks to the in-memory mock
// backend (src/dev/mockBackend.ts), so these tests cover the UI flows, not Rust.
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  reporter: "list",
  use: {
    baseURL: "http://localhost:1420",
    channel: "msedge",
    viewport: { width: 1200, height: 760 },
    locale: "ru-RU",
  },
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
