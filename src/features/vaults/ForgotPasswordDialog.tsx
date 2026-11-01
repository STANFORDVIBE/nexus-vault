import { useState } from "react";
import { TriangleAlertIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { errorMessage } from "@/services/errors";
import { resetForgottenVault } from "@/services/vault";
import { useSessionStore } from "@/store/session";

/** "Forgot password?" link with the honest options: a backup, or start over. */
export function ForgotPasswordDialog() {
  const [open, setOpen] = useState(false);
  const [understood, setUnderstood] = useState(false);
  const [busy, setBusy] = useState(false);
  const refresh = useSessionStore((s) => s.refresh);
  const vaultName = useSessionStore((s) => s.status?.vault.name);

  async function startOver() {
    setBusy(true);
    try {
      const archived = await resetForgottenVault();
      setOpen(false);
      await refresh();
      toast.success("Можно создать хранилище заново", {
        description: `Старый файл не удалён: он сохранён рядом как ${archived}. Если пароль вспомнится, его можно открыть через «Открыть файл хранилища».`,
        duration: 15_000,
      });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Button
        variant="link"
        size="sm"
        className="text-muted-foreground"
        onClick={() => {
          setUnderstood(false);
          setOpen(true);
        }}
      >
        Забыли пароль?
      </Button>

      <Dialog open={open} onOpenChange={(next) => !busy && setOpen(next)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Забыли мастер-пароль?</DialogTitle>
            <DialogDescription>
              Пароль нигде не хранится, поэтому восстановить его нельзя — ни нам, ни кому-либо ещё.
              Без него записи в хранилище «{vaultName}» не расшифровать.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 text-sm">
            <div className="grid gap-2 rounded-md border border-border p-3">
              <span className="font-medium">Есть резервная копия?</span>
              <span className="text-xs leading-relaxed text-muted-foreground">
                Копию можно открыть паролем, который действовал, когда она создавалась. Возможно,
                его вы помните.
              </span>
              <RestoreBackup className="justify-self-start" />
            </div>

            <div className="grid gap-3 rounded-md border border-destructive/30 bg-destructive/5 p-3">
              <span className="font-medium">Начать заново</span>
              <span className="text-xs leading-relaxed text-muted-foreground">
                Будет создано новое пустое хранилище. Старый файл не удаляется — он переименуется и
                останется в той же папке на случай, если пароль вспомнится.
              </span>
              <label className="flex cursor-pointer items-start gap-2.5 text-xs">
                <Checkbox
                  checked={understood}
                  onCheckedChange={(v) => setUnderstood(v === true)}
                  disabled={busy}
                  className="mt-0.5"
                />
                <span>Я понимаю, что записи из этого хранилища будут недоступны</span>
              </label>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setOpen(false)} disabled={busy}>
              Отмена
            </Button>
            <Button
              variant="destructive"
              onClick={() => void startOver()}
              disabled={!understood || busy}
            >
              <TriangleAlertIcon />
              {busy ? "Переименовываем…" : "Начать заново"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
