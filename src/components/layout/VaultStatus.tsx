import { LockIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useAppInfo } from "@/hooks/useAppInfo";
import { useSessionStore } from "@/store/session";
import { useSettingsStore } from "@/store/settings";

/** Sidebar footer: unlocked state, auto-lock timeout, lock button and version. */
export function VaultStatus() {
  const appInfo = useAppInfo();
  const lock = useSessionStore((s) => s.lock);
  const minutes = useSettingsStore((s) => s.settings?.autoLockMinutes);

  return (
    <div className="flex items-center gap-2 px-3 py-3 text-xs">
      <div className="flex min-w-0 flex-1 flex-col gap-1 pl-1">
        <div className="flex items-center gap-2">
          <span className="size-1.5 shrink-0 rounded-full bg-success" aria-hidden="true" />
          <span className="truncate font-medium">Разблокировано</span>
        </div>
        <span className="truncate text-muted-foreground">
          {minutes === undefined
            ? " "
            : minutes === 0
              ? "Автоблокировка выключена"
              : `Автоблокировка: ${minutes} мин`}
          {appInfo && <span className="text-muted-foreground/70"> · v{appInfo.version}</span>}
        </span>
      </div>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => void lock()}
            aria-label="Заблокировать"
          >
            <LockIcon />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="top">Заблокировать · Ctrl+L</TooltipContent>
      </Tooltip>
    </div>
  );
}
