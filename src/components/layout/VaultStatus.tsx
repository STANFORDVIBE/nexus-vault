import { LockOpenIcon } from "lucide-react";
import { useAppInfo } from "@/hooks/useAppInfo";

/**
 * Sidebar footer: vault and lock state plus app version.
 * Until the Security Core exists there is no vault, and the footer says so honestly.
 */
export function VaultStatus() {
  const appInfo = useAppInfo();

  return (
    <div className="flex flex-col gap-1.5 px-4 py-3 text-xs">
      <div className="flex items-center gap-2 text-muted-foreground">
        <span className="size-1.5 rounded-full bg-muted-foreground/60" aria-hidden="true" />
        <span>Хранилище не создано</span>
      </div>
      <div className="flex items-center justify-between text-muted-foreground/80">
        <span className="flex items-center gap-1.5">
          <LockOpenIcon className="size-3" aria-hidden="true" />
          Блокировка недоступна
        </span>
        {appInfo && <span>v{appInfo.version}</span>}
      </div>
    </div>
  );
}
