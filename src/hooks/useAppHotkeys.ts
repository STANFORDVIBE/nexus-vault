import { useEffect } from "react";
import { useNavigate } from "react-router";
import { useSessionStore } from "@/store/session";
import { useUiStore } from "@/store/ui";

const LIST_ROUTES = [
  "/items",
  "/passwords",
  "/notes",
  "/cards",
  "/documents",
  "/favorites",
  "/trash",
  "/folders/",
];

/** Where to open the "new item" editor so the created item shows up in the list. */
export function newItemBase(pathname: string, type: "login" | "note"): string {
  if (type === "note" && (pathname === "/passwords" || pathname === "/notes")) return "/notes";
  if (type === "login" && (pathname === "/notes" || pathname === "/passwords")) return "/passwords";
  if (pathname === "/items" || pathname === "/favorites" || pathname.startsWith("/folders/")) {
    return pathname;
  }
  return type === "note" ? "/notes" : "/passwords";
}

/**
 * In-app shortcuts while unlocked:
 * Ctrl+K / Ctrl+F — search, Ctrl+N — new login, Ctrl+Shift+N — new note, Ctrl+L — lock.
 */
export function useAppHotkeys() {
  const navigate = useNavigate();
  const lock = useSessionStore((s) => s.lock);
  const requestSearchFocus = useUiStore((s) => s.requestSearchFocus);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      // physical key codes, so shortcuts also work with the Russian layout active
      const code = event.code;
      if (code === "KeyK" || code === "KeyF") {
        event.preventDefault();
        // read the live location: the shortcut may fire right after a navigation
        const onList = LIST_ROUTES.some((route) => window.location.pathname.startsWith(route));
        if (!onList) void navigate("/items");
        requestSearchFocus();
      } else if (code === "KeyN") {
        event.preventDefault();
        const type = event.shiftKey ? "note" : "login";
        void navigate(`${newItemBase(window.location.pathname, type)}?new=${type}`);
      } else if (code === "KeyL") {
        event.preventDefault();
        void lock();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate, lock, requestSearchFocus]);
}
