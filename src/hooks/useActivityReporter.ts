import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { reportActivity } from "@/services/vault";
import { useUiStore } from "@/store/ui";

const THROTTLE_MS = 20_000;

/**
 * Tells Rust the user is active (resets the auto-lock timer). IPC calls already count
 * as activity; this covers reading and scrolling without any calls.
 */
export function useActivityReporter() {
  useEffect(() => {
    let last = 0;
    function onActivity() {
      const now = Date.now();
      if (now - last < THROTTLE_MS) return;
      last = now;
      reportActivity().catch(() => {
        // locked in the meantime: the vault-locked event handles the UI
      });
    }
    const events = ["pointerdown", "keydown", "wheel", "pointermove"] as const;
    for (const event of events) window.addEventListener(event, onActivity, { passive: true });
    return () => {
      for (const event of events) window.removeEventListener(event, onActivity);
    };
  }, []);
}

/** Global Ctrl+Shift+L (registered in Rust) opens the window and focuses search. */
export function useQuickSearchEvent(onQuickSearch: () => void) {
  const requestSearchFocus = useUiStore((s) => s.requestSearchFocus);
  useEffect(() => {
    const unlisten = listen("quick-search", () => {
      onQuickSearch();
      requestSearchFocus();
    });
    return () => void unlisten.then((fn) => fn());
  }, [onQuickSearch, requestSearchFocus]);
}
