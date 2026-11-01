import { useState } from "react";
import { DownloadIcon, RefreshCwIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { runUpdate } from "@/features/settings/runUpdate";
import { SettingsRow } from "@/features/settings/SettingsRow";
import { errorMessage } from "@/services/errors";
import { checkUpdate } from "@/services/vault";
import type { Settings, UpdateInfo } from "@/types/vault";

type CheckState = { kind: "idle" } | { kind: "latest" } | { kind: "found"; info: UpdateInfo };

interface UpdateSettingsProps {
  settings: Settings | null;
  save: (patch: Partial<Settings>) => Promise<void>;
}

export function UpdateSettings({ settings, save }: UpdateSettingsProps) {
  const [busy, setBusy] = useState(false);
  const [check, setCheck] = useState<CheckState>({ kind: "idle" });

  async function checkNow() {
    setBusy(true);
    try {
      const info = await checkUpdate();
      setCheck(info ? { kind: "found", info } : { kind: "latest" });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <SettingsRow
        label="Обновления"
        description={
          check.kind === "found"
            ? `Доступна версия ${check.info.version} (у вас ${check.info.currentVersion})`
            : check.kind === "latest"
              ? "У вас последняя версия"
              : "Новые версии берутся с GitHub и проверяются цифровой подписью"
        }
      >
        {check.kind === "found" ? (
          <Button onClick={() => void runUpdate()}>
            <DownloadIcon />
            Обновить
          </Button>
        ) : (
          <Button variant="outline" disabled={busy} onClick={() => void checkNow()}>
            <RefreshCwIcon className={busy ? "animate-spin" : undefined} />
            {busy ? "Проверяем…" : "Проверить обновления"}
          </Button>
        )}
      </SettingsRow>
      {settings && (
        <SettingsRow
          label="Проверять обновления автоматически"
          description="Раз в день приложение узнаёт на GitHub номер последней версии. Данные хранилища никуда не отправляются"
          htmlFor="check-updates"
        >
          <Switch
            id="check-updates"
            checked={settings.checkUpdates}
            onCheckedChange={(checkUpdates) => void save({ checkUpdates })}
          />
        </SettingsRow>
      )}
    </>
  );
}
