import { ShieldAlertIcon } from "lucide-react";
import { AuthLayout } from "@/features/auth/AuthLayout";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { VaultSwitcher } from "@/features/vaults/VaultSwitcher";
import { useSessionStore } from "@/store/session";

/** Shown when the vault file exists but cannot be opened. We never overwrite it here. */
export function CorruptedScreen() {
  const path = useSessionStore((s) => s.status?.vault.path);
  return (
    <AuthLayout
      title="Хранилище повреждено"
      description="Файл хранилища не удаётся открыть: он повреждён или был изменён вне приложения."
      footer={<VaultSwitcher variant="auth" />}
    >
      <div className="flex flex-col gap-4">
        <div className="flex gap-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs leading-relaxed">
          <ShieldAlertIcon className="size-4 shrink-0 text-destructive" />
          <span>
            Мы ничего не удаляли и не перезаписывали. Файл лежит здесь — его можно сохранить для
            разбора: <span className="font-mono break-all">{path}</span>
          </span>
        </div>
        <RestoreBackup variant="default" className="h-10 w-full" />
        <p className="text-center text-xs text-muted-foreground">
          При восстановлении текущий файл сохранится рядом с пометкой «previous»
        </p>
      </div>
    </AuthLayout>
  );
}
