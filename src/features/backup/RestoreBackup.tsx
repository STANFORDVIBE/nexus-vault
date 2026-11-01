import { useState, type FormEvent } from "react";
import { ArchiveRestoreIcon, FileArchiveIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { formatBytes, formatDateTime, ITEMS_FORMS, plural } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { pickBackupFile, restoreBackup } from "@/services/vault";
import { useSessionStore } from "@/store/session";
import type { BackupInfo } from "@/types/vault";

interface RestoreBackupProps {
  variant?: "default" | "outline" | "ghost" | "link";
  label?: string;
  className?: string;
}

/** "Restore from backup" button: pick a .nexus file, then confirm with its password. */
export function RestoreBackup({
  variant = "outline",
  label = "Восстановить из копии",
  className,
}: RestoreBackupProps) {
  const [info, setInfo] = useState<BackupInfo | null>(null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const refresh = useSessionStore((s) => s.refresh);

  async function pick() {
    try {
      const picked = await pickBackupFile();
      if (picked) {
        setPassword("");
        setError(null);
        setInfo(picked);
      }
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!password) return;
    setBusy(true);
    setError(null);
    try {
      const summary = await restoreBackup(password);
      setPassword("");
      setInfo(null);
      await refresh();
      toast.success("Хранилище восстановлено", {
        description: `${summary.items} ${plural(summary.items, ITEMS_FORMS)}. Войдите с мастер-паролем этой копии.`,
      });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Button variant={variant} className={className} onClick={() => void pick()}>
        <ArchiveRestoreIcon />
        {label}
      </Button>

      <Dialog open={info !== null} onOpenChange={(open) => !open && !busy && setInfo(null)}>
        <DialogContent>
          <form onSubmit={(e) => void submit(e)} className="grid gap-5">
            <DialogHeader>
              <DialogTitle>Восстановление из копии</DialogTitle>
              <DialogDescription>
                Текущее хранилище будет заменено содержимым копии. Предыдущая версия сохранится
                рядом как <span className="font-mono text-xs">vault.previous.db</span>.
              </DialogDescription>
            </DialogHeader>

            {info && (
              <div className="flex items-center gap-3 rounded-md border border-border bg-muted/40 p-3">
                <FileArchiveIcon className="size-5 shrink-0 text-brand" strokeWidth={1.75} />
                <div className="min-w-0 text-xs">
                  <div className="truncate text-sm font-medium">{info.fileName}</div>
                  <div className="text-muted-foreground">
                    {formatDateTime(info.createdAt)} · {formatBytes(info.sizeBytes)}
                  </div>
                </div>
              </div>
            )}

            <div className="grid gap-2">
              <Label htmlFor="restore-password">Мастер-пароль этой копии</Label>
              <PasswordInput
                id="restore-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                aria-invalid={error !== null}
                disabled={busy}
                autoFocus
              />
              {error && <p className="text-xs text-destructive">{error}</p>}
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setInfo(null)} disabled={busy}>
                Отмена
              </Button>
              <Button type="submit" disabled={busy || !password}>
                {busy ? "Проверяем копию…" : "Восстановить"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
