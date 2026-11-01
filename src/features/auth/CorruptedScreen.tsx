import { ShieldAlertIcon } from "lucide-react";
import { AuthLayout } from "@/features/auth/AuthLayout";
import { RestoreBackup } from "@/features/backup/RestoreBackup";

/** Shown when the vault file exists but cannot be opened. We never overwrite it here. */
export function CorruptedScreen() {
  return (
    <AuthLayout
      title="Хранилище повреждено"
      description="Файл хранилища не удаётся открыть: он повреждён или был изменён вне приложения."
    >
      <div className="flex flex-col gap-4">
        <div className="flex gap-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs leading-relaxed">
          <ShieldAlertIcon className="size-4 shrink-0 text-destructive" />
          <span>
            Мы ничего не удаляли и не перезаписывали. Файл{" "}
            <span className="font-mono">vault.db</span> лежит в папке данных приложения — его можно
            сохранить для разбора.
          </span>
        </div>
        <RestoreBackup variant="default" className="h-10 w-full" />
        <p className="text-center text-xs text-muted-foreground">
          При восстановлении текущий файл сохранится как vault.previous.db
        </p>
      </div>
    </AuthLayout>
  );
}
