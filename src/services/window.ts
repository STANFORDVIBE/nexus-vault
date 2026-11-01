import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "@tauri-apps/api/core";

/** Syncs the native title bar with the app theme. No-op outside Tauri (tests, browser preview). */
export async function setWindowTheme(theme: "dark" | "light"): Promise<void> {
  if (!isTauri()) return;
  try {
    await getCurrentWindow().setTheme(theme);
  } catch (error) {
    console.warn("Failed to set window theme", error);
  }
}
