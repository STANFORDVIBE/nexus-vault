import { create } from "zustand";
import { setWindowTheme } from "@/services/window";

export type ThemePreference = "dark" | "light" | "system";
export type ResolvedTheme = "dark" | "light";

// Theme is a non-secret UI preference; it must be known before the vault is unlocked,
// so it lives in localStorage rather than in the encrypted vault.
const STORAGE_KEY = "nexus-vault:theme";
const SYSTEM_DARK_QUERY = "(prefers-color-scheme: dark)";

function readPreference(): ThemePreference {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    if (value === "dark" || value === "light" || value === "system") return value;
  } catch {
    // storage unavailable: fall back to the default
  }
  return "dark";
}

function systemTheme(): ResolvedTheme {
  return window.matchMedia(SYSTEM_DARK_QUERY).matches ? "dark" : "light";
}

export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  return preference === "system" ? systemTheme() : preference;
}

function applyTheme(theme: ResolvedTheme) {
  const root = document.documentElement;
  // Switch instantly: without this, every transition-colors element fades between themes
  root.classList.add("theme-switching");
  root.classList.toggle("dark", theme === "dark");
  requestAnimationFrame(() =>
    requestAnimationFrame(() => root.classList.remove("theme-switching")),
  );
  void setWindowTheme(theme);
}

interface ThemeState {
  preference: ThemePreference;
  resolved: ResolvedTheme;
  setPreference: (preference: ThemePreference) => void;
}

export const useThemeStore = create<ThemeState>((set) => {
  const preference = readPreference();
  return {
    preference,
    resolved: resolveTheme(preference),
    setPreference: (next) => {
      try {
        localStorage.setItem(STORAGE_KEY, next);
      } catch {
        // not persisted, still applied for this session
      }
      set({ preference: next, resolved: resolveTheme(next) });
    },
  };
});

/** Applies the current theme to the document and keeps "system" in sync with Windows. */
export function initTheme() {
  applyTheme(useThemeStore.getState().resolved);

  useThemeStore.subscribe((state, prev) => {
    if (state.resolved !== prev.resolved) applyTheme(state.resolved);
  });

  window.matchMedia(SYSTEM_DARK_QUERY).addEventListener("change", () => {
    const { preference } = useThemeStore.getState();
    if (preference === "system") useThemeStore.setState({ resolved: resolveTheme("system") });
  });
}
