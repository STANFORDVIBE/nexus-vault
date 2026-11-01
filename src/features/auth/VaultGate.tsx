import { useEffect, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { CorruptedScreen } from "@/features/auth/CorruptedScreen";
import { OnboardingScreen } from "@/features/auth/OnboardingScreen";
import { UnlockScreen } from "@/features/auth/UnlockScreen";
import { plural } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { useSessionStore } from "@/store/session";
import { useSettingsStore } from "@/store/settings";

/** Subscribes to backend events for the whole app lifetime. */
function useBackendEvents() {
  const markLocked = useSessionStore((s) => s.markLocked);

  useEffect(() => {
    const unlisteners = [
      listen("vault-locked", () => markLocked()),
      listen("clipboard-cleared", () => toast("Буфер обмена очищен")),
      listen("auto-backup-failed", () =>
        toast.error("Не удалось сделать автоматическую копию", {
          description: "Проверьте папку для автокопий в настройках",
        }),
      ),
    ];
    return () => {
      for (const promise of unlisteners) void promise.then((unlisten) => unlisten());
    };
  }, [markLocked]);
}

/**
 * Picks the screen by vault state. Everything inside `children` exists only while
 * unlocked: locking unmounts it, so no decrypted data stays in React state.
 */
export function VaultGate({ children }: { children: ReactNode }) {
  const status = useSessionStore((s) => s.status);
  const refresh = useSessionStore((s) => s.refresh);
  const loadSettings = useSettingsStore((s) => s.load);
  const state = status?.state;

  useBackendEvents();

  useEffect(() => {
    refresh().catch((e: unknown) => toast.error(errorMessage(e)));
  }, [refresh]);

  useEffect(() => {
    // drop ?item=…&edit=1 so unlocking does not reopen an editor from before the lock
    if (state === "locked") window.history.replaceState(null, "", window.location.pathname);
    if (state !== "unlocked") return;
    loadSettings().catch((e: unknown) => toast.error(errorMessage(e)));
    const corrupted = status?.corruptedItems ?? 0;
    if (corrupted > 0) {
      toast.warning(
        `${corrupted} ${plural(corrupted, ["запись не удалось", "записи не удалось", "записей не удалось"])} расшифровать`,
        { description: "Они пропущены и не изменены. Восстановите их из резервной копии." },
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state]);

  if (!status) return <div className="h-full bg-background" />;
  switch (status.state) {
    case "uninitialized":
      return <OnboardingScreen />;
    case "locked":
      return <UnlockScreen />;
    case "corrupted":
      return <CorruptedScreen />;
    case "unlocked":
      return children;
  }
}
